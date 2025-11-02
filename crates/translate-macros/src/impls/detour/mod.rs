pub(crate) mod detour_fn;
pub(crate) mod detour_trait;

use quote::format_ident;
use syn::{Attribute, Expr, Ident, LitStr};

struct DetourAttr {
    dll: String,
    symbol: String,
    export: Option<String>,
    fallback: Option<Expr>,
    calling_convention: Option<String>,
}

fn parse_detour_attr(attr: &Attribute) -> syn::Result<Option<DetourAttr>> {
    if !attr.path().is_ident("detour") {
        return Ok(None);
    }

    let mut dll: Option<String> = None;
    let mut symbol: Option<String> = None;
    let mut export: Option<String> = None;
    let mut fallback: Option<Expr> = None;
    let mut calling_convention: Option<String> = None;

    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            let key = ident.to_string();
            let buf = meta.value()?;

            match buf.parse::<LitStr>() {
                Ok(litstr) => {
                    match key.as_str() {
                        "dll" => dll = Some(litstr.value()),
                        "symbol" => symbol = Some(litstr.value()),
                        "export" => export = Some(litstr.value()),
                        "fallback" => match syn::parse_str::<Expr>(&litstr.value()) {
                            Ok(expr) => fallback = Some(expr),
                            Err(e) => syn_bail!(litstr, "解析 fallback 表达式失败: {e}"),
                        },
                        "calling_convention" => {
                            calling_convention = Some(litstr.value());
                        }
                        key => syn_bail!(litstr, "未知的key: {key}"),
                    }
                    return Ok(());
                }
                Err(_) => syn_bail!(attr, "detour 属性的值必须使用字符串字面量"),
            }
        }
        Ok(())
    })?;

    match (dll, symbol) {
        (Some(dll), Some(symbol)) => Ok(Some(DetourAttr {
            dll,
            symbol,
            export,
            fallback,
            calling_convention,
        })),
        _ => syn_bail!(
            attr.path(),
            "detour 属性必须包含 dll 和 symbol 两个字符串字面量"
        ),
    }
}

fn parse_detour_attrs<'a>(
    attrs: impl Iterator<Item = &'a Attribute>,
) -> syn::Result<Option<DetourAttr>> {
    let mut detour_meta: Option<DetourAttr> = None;
    for attr in attrs {
        match parse_detour_attr(attr) {
            Ok(Some(parsed)) => {
                detour_meta = Some(parsed);
                break;
            }
            Ok(None) => { /* 这个 attr 不是 detour，继续 */ }
            Err(e) => syn_bail!(attr, "{e}"),
        }
    }

    Ok(detour_meta)
}

fn generate_detour_ident(ident: &Ident) -> Ident {
    let static_name = format!("HOOK_{}", ident.to_string().to_uppercase());
    format_ident!("{}", static_name)
}
