use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{LitInt, LitStr};

pub fn byte_slice(input: TokenStream) -> syn::Result<TokenStream> {
    // 只接受一个字符串字面量
    let lit = syn::parse2::<LitStr>(input)?;
    let s = lit.value();

    // 基本检查：不能为空
    if s.is_empty() {
        syn_bail!(
            lit,
            "参数为空字符串；示例：to_byte_slice![\"0C 00 0E 00 90 7F AC\"];"
        );
    }

    // 不允许最前和最后有空格
    if s.starts_with(' ') || s.ends_with(' ') {
        syn_bail!(
            lit,
            "参数不能以空格开头或结尾；请确保字符串精确格式，例如：\"0C 00 0E 00\"",
        );
    }

    // 不允许出现连续两个或更多空格（必须严格单空格分隔）
    if s.contains("  ") {
        syn_bail!(lit, "参数中不允许有连续空格；字节之间必须用单个空格分隔。");
    }

    // 检查并解析每个 token（按空格分割）
    let parts: Vec<&str> = s.split(' ').collect();
    if parts.is_empty() {
        syn_bail!(
            lit,
            "参数解析失败；请提供至少一个字节，例如：\"FF\" 或 \"0C 00\"。"
        );
    }

    // 存放生成的字面量
    let mut lits: Vec<LitInt> = Vec::with_capacity(parts.len());

    for (i, part) in parts.iter().enumerate() {
        // 每个 part 必须恰好长度为 2
        if part.len() != 2 {
            syn_bail!(
                lit,
                "第 {} 个字节长度错误（必须为两位十六进制字符），收到 `{}`。示例格式：\"0C 00 0E\"",
                i + 1,
                part,
            );
        }
        // 两个字符必须都是十六进制字符
        let mut ok = true;
        for ch in part.chars() {
            if !ch.is_ascii_hexdigit() {
                ok = false;
                break;
            }
        }
        if !ok {
            syn_bail!(
                lit,
                "第 {} 个字节包含非十六进制字符：`{}`。只能包含 0-9 A-F a-f。",
                i + 1,
                part
            );
        }

        // 解析成 u8（安全，前面检查了）
        let value = match u8::from_str_radix(part, 16) {
            Ok(v) => v,
            Err(_) => {
                syn_bail!(lit, "无法解析第 {} 个字节 `{}` 为 hex。", i + 1, part);
            }
        };

        // 以十六进制带 u8 后缀的字面量形式创建 LitInt（例如 "0x0C_u8"）
        // 使用 0x{:02X}u8 形式更直观
        let lit_text = format!("0x{value:02X}u8");
        let lit = LitInt::new(&lit_text, Span::call_site());
        lits.push(lit);
    }

    // 生成数组字面量，例如: [0x0C_u8, 0x00_u8, ...]
    let output = quote! {
        [ #(#lits),* ]
    };

    Ok(output)
}
