/// 检查一个字节是否是有效的 Shift-JIS 高位（第一）字节。
pub fn is_sjis_high_byte(b: u8) -> bool {
    (0x81..=0x9F).contains(&b) || (0xE0..=0xFC).contains(&b)
}
