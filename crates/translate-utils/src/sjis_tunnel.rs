use anyhow::{Context, Ok, Result, bail};
use encoding_rs::SHIFT_JIS;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::jis0208::is_jis0208;
use crate::utils;

// --- 常量 (编码器和解码器共用) ---
const LOW_BYTES_TO_AVOID: [u8; 5] = [b'\t', b'\n', b'\r', b' ', b','];
const PER_ROW: usize = 0x40 - LOW_BYTES_TO_AVOID.len() - 1;

/// 封装了 SJIS 隧道编码和解码的逻辑。
/// 注意：不支持除了基本平面以外的字符，比如emoji
#[derive(Default)]
pub struct SjisTunnel {
    /// 将原始字符映射到其生成的隧道表示
    forward_map: HashMap<char, u16>,
    /// 按首次被隧道化的顺序存储原始字符的 UTF-16 码位。
    /// 直接存储 u16 以优化 decode 性能
    reverse_map: Vec<u16>,
}

impl SjisTunnel {
    /// 创建一个新的、空的 `SjisTunnel` 实例。
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用一个已有的映射表来创建一个新的 `SjisTunnel` 实例。
    pub fn from_mapping_table(table: &[u8]) -> Result<Self> {
        if !table.len().is_multiple_of(2) {
            bail!("映射表的字节数必须为偶数。");
        }

        let mut tunnel = Self::new();
        let mut i = 0;
        while i < table.len() {
            let char_code = u16::from_le_bytes([table[i], table[i + 1]]);
            // 仍然需要转换为char来填充forward_map
            if let Some(c) = std::char::from_u32(char_code as u32) {
                tunnel.get_or_create_tunnel_char(c)?;
            } else {
                bail!("映射表中包含无效的字符码: {char_code}");
            }
            i += 2;
        }
        Ok(tunnel)
    }

    /// 使用 SJIS 隧道策略编码一个字符串。
    pub fn encode(&mut self, text: &str) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        for c in text.chars() {
            if c.is_ascii() || is_jis0208(c) {
                let mut buf = [0; 4];
                let s = c.encode_utf8(&mut buf);
                let (cow, _, had_errors) = SHIFT_JIS.encode(s);

                assert!(!had_errors, "通过 `is_jis0208` 但无法编码");
                result.extend_from_slice(&cow);
            } else {
                let tunnel_char = self.get_or_create_tunnel_char(c)?;
                result.extend_from_slice(&tunnel_char.to_be_bytes());
            }
        }
        Ok(result)
    }

    /// 使用 SJIS 隧道策略解码一个字节切片。
    pub fn decode(&self, input: &[u8]) -> Vec<u16> {
        utils::mapping(input, |tunnel_char: u16| {
            let idx = Self::tunnel_char_to_mapping_index(tunnel_char)?;
            self.reverse_map.get(idx).copied()
        })
    }

    /// 使用 SJIS 隧道策略解码一个字节切片并将其转换为 String
    pub fn decode_to_string(&self, input: &[u8]) -> Result<String> {
        String::from_utf16(&self.decode(input)).context("解码后的数据不是有效的 UTF-16")
    }

    /// 以字节向量的形式返回映射表。
    pub fn get_mapping_table(&self) -> Vec<u8> {
        let mut table = Vec::with_capacity(self.reverse_map.len() * 2);
        // 直接遍历u16，无需转换
        for &code in &self.reverse_map {
            table.extend_from_slice(&code.to_le_bytes());
        }
        table
    }

    /// 将映射表写入到指定的文件路径。
    pub fn write_mapping_table_to_path<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let table = self.get_mapping_table();
        let mut file = File::create(path)?;
        file.write_all(&table)
    }

    /// 返回已经使用的隧道字符数量
    pub fn len(&self) -> usize {
        self.reverse_map.len()
    }

    /// 是否没有使用任何隧道字符
    pub fn is_empty(&self) -> bool {
        self.reverse_map.is_empty()
    }

    /// 获取 `orig_char` 已有的隧道字符，或者创建一个新的。
    fn get_or_create_tunnel_char(&mut self, orig_char: char) -> Result<u16> {
        if let Some(&sjis_char) = self.forward_map.get(&orig_char) {
            return Ok(sjis_char);
        }

        let sjis_idx = self.reverse_map.len();
        if sjis_idx >= 0x3B * PER_ROW {
            bail!("SJIS 隧道数量超出上限，无法映射字符 '{orig_char}'。");
        }

        let high_sjis_idx = sjis_idx / PER_ROW;
        let low_sjis_idx = sjis_idx % PER_ROW;

        let high_byte = if high_sjis_idx < 0x1F {
            0x81 + high_sjis_idx
        } else {
            0xE0 + (high_sjis_idx - 0x1F)
        } as u8;

        let mut low_byte = (1 + low_sjis_idx) as u8;
        for &avoid_byte in LOW_BYTES_TO_AVOID.iter() {
            if low_byte >= avoid_byte {
                low_byte += 1;
            }
        }

        let tunnel_char = ((high_byte as u16) << 8) | (low_byte as u16);
        self.forward_map.insert(orig_char, tunnel_char);
        self.reverse_map.push(orig_char as u16);

        Ok(tunnel_char)
    }

    /// 将一个双字节的隧道字符转换回它在映射表中的索引。
    fn tunnel_char_to_mapping_index(t: u16) -> Option<usize> {
        let high = (t >> 8) as u8;
        let low = (t & 0xFF) as u8;

        if !utils::is_sjis_high_byte(high) || low == 0 || low >= 0x40 {
            return None;
        }

        let high_idx = if high < 0xA0 {
            (high - 0x81) as usize
        } else {
            0x1F + (high - 0xE0) as usize
        };

        let mut low_idx_i32 = low as i32;
        for &avoid_byte in LOW_BYTES_TO_AVOID.iter().rev() {
            if low_idx_i32 > avoid_byte as i32 {
                low_idx_i32 -= 1;
            }
        }
        low_idx_i32 -= 1;

        if low_idx_i32 < 0 {
            None
        } else {
            Some(high_idx * PER_ROW + (low_idx_i32 as usize))
        }
    }
}
