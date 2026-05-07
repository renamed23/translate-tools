use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

/// 单个“特性化 Hook 列表”条目。
///
/// 该结构对应 `featured_hook_lists.json` 中某个 `cfg` 表达式下的配置项，
/// 用于分别描述：
/// - 在该 `cfg` 条件成立时，哪些 Hook trait 需要被视为“已提供特化实现”；
/// - 在该 `cfg` 条件成立时，哪些 detour 函数需要自动加入启用/停用列表。
///
/// 之所以把 trait 与函数分开，是因为两者服务的宏展开场景不同：
/// - `derive_default_hook` 只关心 trait 维度，用它决定哪些默认 impl 需要跳过；
/// - `generate_hook_lists` 只关心 detour 函数维度，用它生成 enable/disable 逻辑。
#[derive(Default, Deserialize)]
pub struct FeaturedHookEntry {
    /// 在当前 `cfg` 条件下被视为“已有特化实现”的 Hook trait 名称列表。
    ///
    /// JSON 字段名为 `trait`，例如：
    /// `{ "trait": ["ReadFile"] }`
    #[serde(default, rename = "trait")]
    pub traits: Vec<String>,

    /// 在当前 `cfg` 条件下需要自动参与启用/停用的 detour 函数名列表。
    ///
    /// JSON 字段名为 `fn`，例如：
    /// `{ "fn": ["CreateFileA", "CreateFileW"] }`
    #[serde(default, rename = "fn")]
    pub fns: Vec<String>,
}

/// 全部“特性化 Hook 列表”的顶层结构。
///
/// 外层 `HashMap` 的 key 是一段可直接嵌入 `#[cfg(...)]` 的条件表达式字符串，
/// value 是该条件对应的 [`FeaturedHookEntry`]。
///
/// 例如：
/// ```json
/// {
///   "feature = \"text_hook\"": {
///     "fn": ["CreateFontA", "CreateFontW"]
///   }
/// }
/// ```
#[derive(Default, Deserialize)]
pub struct FeaturedHookLists(pub HashMap<String, FeaturedHookEntry>);

/// 将 JSON 中声明的 `cfg` 字符串解析成可插入宏展开结果的 token。
///
/// 输入应当是 `feature = "xxx"`、`all(...)`、`any(...)` 这类合法的 `cfg`
/// 内部表达式，而不是完整的 `#[cfg(...)]` 属性。
pub fn parse_cfg_expr(cfg_key: &str) -> syn::Result<TokenStream> {
    cfg_key
        .parse()
        .map_err(|e| syn_err2!("无法解析 cfg key `{cfg_key}`: {e}"))
}

/// 基于 [`FeaturedHookLists`] 构建 “trait 名 -> cfg 条件列表” 的反向索引。
///
/// 这个映射主要给 `derive_default_hook` 使用：
/// - 如果某个 trait 出现在 featured 配置里，说明它在某些 feature 组合下
///   会由专门实现接管；
/// - 那么默认实现就需要在这些条件下被禁用，避免和专门实现冲突。
///
/// 返回值中的 `Vec<TokenStream>` 表示：某个 trait 可能被多个不同的 `cfg`
/// 条件声明为 featured，因此需要把这些条件累计起来，后续统一生成
/// `#[cfg(not(any(...)))]`。
pub fn build_featured_trait_cfg_map(
    featured: &FeaturedHookLists,
) -> syn::Result<HashMap<String, Vec<TokenStream>>> {
    let mut trait_cfg_map: HashMap<String, Vec<TokenStream>> = HashMap::new();

    for (cfg_key, entry) in &featured.0 {
        let cfg_ts = parse_cfg_expr(cfg_key)?;

        for trait_name in &entry.traits {
            trait_cfg_map
                .entry(trait_name.clone())
                .or_default()
                .push(cfg_ts.clone());
        }
    }

    Ok(trait_cfg_map)
}

/// 根据一组 `cfg` 条件构造 `#[cfg(not(any(...)))]` 属性。
///
/// 当传入切片为空时，返回空 token，表示调用方无需附加任何条件编译限制。
/// 这通常意味着该 trait 没有被任何 featured 配置声明覆盖，默认实现应始终生成。
pub fn build_cfg_not_any(cfgs: &[TokenStream]) -> TokenStream {
    if cfgs.is_empty() {
        return quote! {};
    }

    quote! {
        #[cfg(not(any(#(#cfgs),*)))]
    }
}
