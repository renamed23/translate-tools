pub(crate) mod detour_fn;
pub(crate) mod detour_trait;

use convert_case::{Case, Casing};
use quote::format_ident;
use syn::{Attribute, Expr, Ident, LitStr};

pub struct DetourAttr {
    pub dll: String,
    pub symbol: String,
    pub export: Option<String>,
    pub fallback: Option<Expr>,
    pub calling_convention: Option<String>,
    pub enable_hook_guard: bool,
}

pub fn parse_detour_attr(attr: &Attribute) -> syn::Result<Option<DetourAttr>> {
    if !attr.path().is_ident("detour") {
        return Ok(None);
    }

    let mut dll: Option<String> = None;
    let mut symbol: Option<String> = None;
    let mut export: Option<String> = None;
    let mut fallback: Option<Expr> = None;
    let mut calling_convention: Option<String> = None;
    let mut enable_hook_guard: bool = true;

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
                        "enable_hook_guard" => match litstr.value().parse::<bool>() {
                            Ok(enable) => enable_hook_guard = enable,
                            Err(e) => syn_bail!(litstr, "解析 enable_hook_guard 失败: {e}"),
                        },
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
            enable_hook_guard,
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

pub fn generate_detour_ident(ident: &Ident) -> Ident {
    let static_name = format!(
        "HOOK_{}",
        ident.to_string().to_case(Case::Snake).to_uppercase()
    );
    format_ident!("{static_name}")
}
