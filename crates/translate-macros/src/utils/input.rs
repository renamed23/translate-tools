use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};

/// 仅包含单个路径字面量的宏输入。
pub struct SinglePath {
    /// 传入的路径字符串。
    pub path: LitStr,
}

impl Parse for SinglePath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse()?,
        })
    }
}

/// 由两个逗号分隔的路径字面量组成的宏输入。
pub struct CommaSeparatedPaths {
    /// 逗号左侧的路径字符串。
    pub left: LitStr,
    /// 逗号右侧的路径字符串。
    pub right: LitStr,
}

impl Parse for CommaSeparatedPaths {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let left = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let right = input.parse()?;
        Ok(Self { left, right })
    }
}

/// 由 `=>` 分隔的两个路径字面量组成的宏输入。
pub struct ArrowSeparatedPaths {
    /// `=>` 左侧的路径字符串。
    pub left: LitStr,
    /// `=>` 右侧的路径字符串。
    pub right: LitStr,
}

impl Parse for ArrowSeparatedPaths {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let left = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let right = input.parse()?;
        Ok(Self { left, right })
    }
}
