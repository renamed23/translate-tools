use anyhow::{Context, Result, anyhow, bail};
use encoding_rs::SHIFT_JIS;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

use crate::jis0208::is_jis0208;

/// ReplacementPool：管理替身池与一对一映射
pub struct ReplacementPool {
    pool: Vec<char>,                   // 经验证过的候选替身
    free: VecDeque<char>,              // 可用的替身
    orig_to_repl: HashMap<char, char>, // 原字符 -> 替身
    repl_to_orig: HashMap<char, char>, // 替身 -> 原字符
}

impl ReplacementPool {
    /// 从 JSON 文件加载替身池
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = fs::read_to_string(&path)
            .with_context(|| format!("读取替身池文件失败: {}", path.as_ref().display()))?;
        Self::from_string(&s)
    }

    /// 从 JSON 字符串加载替身池
    pub fn from_string(json_str: &str) -> Result<Self> {
        let v: Value = serde_json::from_str(json_str).context("解析替身池 JSON 失败")?;

        let raw: Vec<String> = if v.is_object() && v.get("pool").is_some() {
            serde_json::from_value(v["pool"].clone())
                .context("从对象的 `pool` 字段解析替身池失败")?
        } else if v.is_array() {
            serde_json::from_value(v).context("从数组解析替身池失败")?
        } else {
            return Err(anyhow!(
                "替身池 JSON 格式不支持（应为数组或包含 pool 字段的对象）"
            ));
        };

        let mut seen = HashSet::new();
        let mut pool = Vec::with_capacity(raw.len());
        for s in raw {
            if s.chars().count() != 1 {
                eprintln!("跳过非单字符池项: {s:?}");
                continue;
            }
            let ch = s.chars().next().unwrap();
            if !is_jis0208(ch) {
                eprintln!("跳过非 JIS X 0208 的池项: '{ch}'");
                continue;
            }
            if seen.insert(ch) {
                pool.push(ch);
            } else {
                eprintln!("跳过重复池项: '{ch}'");
            }
        }

        if pool.is_empty() {
            bail!("替身池加载后为空，请检查 pool 数据。");
        }

        let free = VecDeque::from(pool.clone());

        Ok(Self {
            pool,
            free,
            orig_to_repl: HashMap::new(),
            repl_to_orig: HashMap::new(),
        })
    }

    /// 重置所有映射（保留池内容）
    pub fn reset(&mut self) {
        self.orig_to_repl.clear();
        self.repl_to_orig.clear();
        self.free.clear();
        self.free.extend(self.pool.iter().cloned());
    }

    /// 为 orig 分配或返回已有替身
    pub fn get(&mut self, orig: char) -> Result<char> {
        if let Some(&r) = self.orig_to_repl.get(&orig) {
            return Ok(r);
        }
        if let Some(candidate) = self.free.pop_front() {
            assert!(!self.repl_to_orig.contains_key(&candidate));

            self.orig_to_repl.insert(orig, candidate);
            self.repl_to_orig.insert(candidate, orig);
            return Ok(candidate);
        }

        Err(anyhow!("替身池已耗尽，无法为 '{orig}' 分配替身"))
    }

    /// 把字符串映射为 JIS X 0208 兼容字符串，返回映射结果
    pub fn map_text(&mut self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len());

        for ch in text.chars() {
            if ch.is_ascii() || is_jis0208(ch) {
                out.push(ch);
                continue;
            }
            let repl = self.get(ch)?;
            out.push(repl);
        }

        Ok(out)
    }

    /// 生成 charmap 映射表（替身字符 -> 原字符）
    pub fn generate_charmap(&self) -> HashMap<char, char> {
        self.repl_to_orig.clone()
    }

    /// 将 charmap 直接写入目标路径（JSON）
    pub fn write_charmap_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let map = self.generate_charmap();
        let json = serde_json::to_string_pretty(&map)?;
        fs::write(&path, json)
            .with_context(|| format!("写入 charmap 文件失败: {}", path.as_ref().display()))?;
        Ok(())
    }

    pub fn pool_chars(&self) -> &[char] {
        &self.pool
    }

    pub fn current_mapping(&self) -> &HashMap<char, char> {
        &self.orig_to_repl
    }
}

/// 替身池构建器
#[derive(Default, Debug, Clone)]
pub struct PoolBuilder {
    pool: HashSet<char>,
}

impl PoolBuilder {
    /// 生成初始的Shift-JIS字符池
    pub fn generate_shiftjis_pool(&mut self, cp932: bool) -> Result<()> {
        // Shift-JIS字符范围定义
        let sjis_ranges = [
            (0x3041, 0x3096), // 平假名 (Hiragana)
            (0x30A1, 0x30FA), // 片假名 (Katakana)
            (0x30FD, 0x30FE), // ヽ-ヾ
            (0x31F0, 0x31FF), // 片假名扩展
            // (0xFF66, 0xFF9F), // 半角片假名
            (0x4E00, 0x9FFF), // CJK统一汉字 (日本汉字)
            (0x3400, 0x4DBF), // CJK扩展A (兼容汉字)
        ];

        // 生成所有有效的Shift-JIS字符
        for &(start, end) in &sjis_ranges {
            for code in start..=end {
                if let Some(ch) = std::char::from_u32(code) {
                    // 跳过不支持jis0208的字符
                    if !cp932 && !is_jis0208(ch) {
                        continue;
                    }

                    // 验证字符编码
                    let mut tmp = [0u8; 4];
                    let s = ch.encode_utf8(&mut tmp);
                    let (bytes_cow, _, had_errors) = SHIFT_JIS.encode(s);
                    let bytes = bytes_cow.as_ref();

                    if !had_errors && bytes.len() == 2 {
                        self.pool.insert(ch);
                    }
                }
            }
        }

        if self.pool.is_empty() {
            bail!("初始字符池为空");
        }

        Ok(())
    }

    /// 从文本内容中剔除已使用的中文字符
    pub fn exclude_used_chars(&mut self, text: &str) {
        for ch in text.chars() {
            self.pool.remove(&ch);
        }
    }

    /// 将替身池保存到文件
    pub fn write_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(&json!({"pool": self.to_vec()}))?;

        fs::write(&path, json)
            .with_context(|| format!("写入替身池文件失败: {}", path.as_ref().display()))?;

        Ok(())
    }

    /// 将替身池转换为排序后的字符向量
    pub fn to_vec(&self) -> Vec<char> {
        let mut chars_vec: Vec<char> = self.pool.iter().cloned().collect();
        chars_vec.sort();
        chars_vec.reverse();
        chars_vec
    }

    /// 获取当前池大小
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// 检查池是否为空
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}
