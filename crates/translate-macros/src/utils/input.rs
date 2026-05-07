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

/// 由 1..N 个逗号分隔的路径字面量组成的宏输入。
pub struct MultiPaths {
    /// 有序的路径字符串列表。第一个路径是基底，后续路径按序合并。
    pub paths: Vec<LitStr>,
}

impl Parse for MultiPaths {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut paths = Vec::new();

        let first: LitStr = input.parse()?;
        paths.push(first);

        while input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            let path: LitStr = input.parse()?;
            paths.push(path);
        }

        Ok(Self { paths })
    }
}
