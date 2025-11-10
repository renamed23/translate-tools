use encoding_rs::{GBK, SHIFT_JIS};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumString};

/// 支持的编码类型
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, SerializeDisplay, DeserializeFromStr, EnumString, Display,
)]
pub enum EncodingType {
    ShiftJIS,
    GBK,
    CP932,
}

impl EncodingType {
    /// 获取对应的编码器
    pub fn encoder(&self) -> &'static encoding_rs::Encoding {
        match self {
            // encoding_rs::SHIFT_JIS实际上等同于CP932
            EncodingType::ShiftJIS | EncodingType::CP932 => SHIFT_JIS,
            EncodingType::GBK => GBK,
        }
    }

    /// 检查字符是否在该编码中可用
    pub fn contains_char(&self, ch: char) -> bool {
        match self {
            EncodingType::ShiftJIS => {
                // 使用 JIS X 0208 标准检查
                crate::jis0208::is_jis0208(ch) || ch.is_ascii()
            }
            EncodingType::CP932 | EncodingType::GBK => {
                // 使用 encoding_rs 检查
                let encoder = self.encoder();
                let mut buffer = [0u8; 4];
                let encoded = ch.encode_utf8(&mut buffer);
                let (_, _, had_errors) = encoder.encode(encoded);
                !had_errors
            }
        }
    }

    /// 获取该编码建议的字符范围
    pub fn suggested_ranges(&self) -> Vec<(u32, u32)> {
        match self {
            EncodingType::ShiftJIS | EncodingType::CP932 => vec![
                (0x3041, 0x3096), // 平假名
                (0x30A1, 0x30FA), // 片假名
                (0x30FD, 0x30FE), // ヽ-ヾ
                (0x31F0, 0x31FF), // 片假名扩展
                (0x4E00, 0x9FFF), // CJK统一汉字
                (0x3400, 0x4DBF), // CJK扩展A
            ],
            EncodingType::GBK => vec![
                (0x4E00, 0x9FFF), // CJK统一汉字
                (0x3400, 0x4DBF), // CJK扩展A
                (0x2000, 0x206F), // 常用标点
                (0x3000, 0x303F), // CJK符号和标点
            ],
        }
    }

    /// 用于 `MultiByteToWideChar` 的代码页
    pub fn code_page(&self) -> u32 {
        match self {
            EncodingType::ShiftJIS | EncodingType::CP932 => 932,
            EncodingType::GBK => 936,
        }
    }
}
