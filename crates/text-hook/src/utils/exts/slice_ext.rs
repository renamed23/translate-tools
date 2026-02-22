use std::{borrow::Cow, ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use windows_sys::Win32::Globalization::CP_UTF8;

use crate::{
    code_cvt::{mapping_impl, multi_byte_to_wide_char_impl, wide_char_to_multi_byte_impl},
    constant::ANSI_CODE_PAGE,
};

pub trait ByteSliceExt {
    /// 根据指定的 `code_page` 将字节序列转换为宽字符向量。
    fn to_wide(&self, code_page: u32) -> Vec<u16>;

    /// 根据指定的 `code_page` 将字节序列转换为以 null 结尾的宽字符向量。
    fn to_wide_null(&self, code_page: u32) -> Vec<u16>;

    /// 转换为 UTF-16 编码的宽字符。
    fn to_wide_utf8(&self) -> Vec<u16> {
        self.to_wide(CP_UTF8)
    }

    /// 按照常量 ANSI 代码页转换为宽字符。
    fn to_wide_ansi(&self) -> Vec<u16> {
        self.to_wide(ANSI_CODE_PAGE)
    }

    /// 转换为以 null 结尾的 UTF-16 宽字符。
    fn to_wide_null_utf8(&self) -> Vec<u16> {
        self.to_wide_null(CP_UTF8)
    }

    /// 按照常量 ANSI 代码页转换为以 null 结尾的宽字符。
    fn to_wide_null_ansi(&self) -> Vec<u16> {
        self.to_wide_null(ANSI_CODE_PAGE)
    }

    /// 尝试将字节序列解析为 UTF-8 字符串切片。
    fn to_str(&self) -> crate::Result<&str>;

    /// 将字节序列损失地转换为 UTF-8 `Cow<str>`（替换无效字符）。
    fn to_string_lossy(&self) -> Cow<'_, str>;

    /// 在当前字节序列末尾追加一个 `0u8` 终止符。
    fn with_null(&self) -> Vec<u8>;
}

impl ByteSliceExt for [u8] {
    fn to_wide(&self, code_page: u32) -> Vec<u16> {
        multi_byte_to_wide_char_impl(self, code_page, false).unwrap_or_default()
    }

    fn to_wide_null(&self, code_page: u32) -> Vec<u16> {
        multi_byte_to_wide_char_impl(self, code_page, true).unwrap_or_default()
    }

    fn to_str(&self) -> crate::Result<&str> {
        Ok(str::from_utf8(self)?)
    }

    fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self)
    }

    fn with_null(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.len() + 1);
        v.extend_from_slice(self);
        v.push(0);
        v
    }
}

pub trait WideSliceExt {
    /// 根据指定的 `code_page` 将宽字符转换为多字节字节向量。
    fn to_multi_byte(&self, code_page: u32) -> Vec<u8>;

    /// 根据指定的 `code_page` 将宽字符转换为以 null 结尾的多字节字节向量。
    fn to_multi_byte_null(&self, code_page: u32) -> Vec<u8>;

    /// 转换为 UTF-8 编码的字节向量。
    fn to_utf8(&self) -> Vec<u8> {
        self.to_multi_byte(CP_UTF8)
    }

    /// 按照常量 ANSI 代码页转换为多字节字节。
    fn to_ansi(&self) -> Vec<u8> {
        self.to_multi_byte(ANSI_CODE_PAGE)
    }

    /// 转换为以 null 结尾的 UTF-8 字节。
    fn to_utf8_null(&self) -> Vec<u8> {
        self.to_multi_byte_null(CP_UTF8)
    }

    /// 按照常量 ANSI 代码页转换为以 null 结尾的多字节字节。
    fn to_ansi_null(&self) -> Vec<u8> {
        self.to_multi_byte_null(ANSI_CODE_PAGE)
    }

    /// 执行字符映射转换。
    fn mapping(&self) -> Vec<u16>;

    /// 执行带 null 终止符的字符映射转换。
    fn mapping_null(&self) -> Vec<u16>;

    /// 尝试转换为 `String`。
    fn to_string(&self) -> crate::Result<String>;

    /// 损失地转换为 `String`（替换无效 UTF-16 代理对）。
    fn to_string_lossy(&self) -> String;

    /// 在当前宽字符序列末尾追加一个 `0u16` 终止符。
    fn with_null(&self) -> Vec<u16>;

    /// 验证宽字符序列是否包含无效字符 (如 U+FFFD)。
    fn valid(&self) -> crate::Result<&[u16]>;

    /// 转换为 `OsString`。
    fn to_os_string(&self) -> OsString;

    /// 转换为 `PathBuf`。
    fn to_path_buf(&self) -> PathBuf;

    /// 文本补丁：查找对应的翻译/映射文本。
    #[cfg(all(feature = "text_patch", not(feature = "text_extracting")))]
    fn lookup(&self) -> crate::Result<Vec<u16>>;

    /// 文本补丁：查找对应的文本，若不存在则添加。
    #[cfg(feature = "text_patch")]
    fn lookup_or_add_item(&self) -> crate::Result<Vec<u16>>;

    /// 文本补丁：查找并返回以 null 结尾的文本。
    #[cfg(all(feature = "text_patch", not(feature = "text_extracting")))]
    fn lookup_null(&self) -> crate::Result<Vec<u16>>;

    /// 文本补丁：查找或添加并返回以 null 结尾的文本。
    #[cfg(feature = "text_patch")]
    fn lookup_or_add_item_null(&self) -> crate::Result<Vec<u16>>;
}

impl WideSliceExt for [u16] {
    fn to_multi_byte(&self, code_page: u32) -> Vec<u8> {
        wide_char_to_multi_byte_impl(self, code_page, false).unwrap_or_default()
    }

    fn to_multi_byte_null(&self, code_page: u32) -> Vec<u8> {
        wide_char_to_multi_byte_impl(self, code_page, true).unwrap_or_default()
    }

    fn mapping(&self) -> Vec<u16> {
        mapping_impl(self, false)
    }

    fn mapping_null(&self) -> Vec<u16> {
        mapping_impl(self, true)
    }

    fn to_string(&self) -> crate::Result<String> {
        Ok(String::from_utf16(self)?)
    }

    fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(self)
    }

    fn with_null(&self) -> Vec<u16> {
        let mut v = Vec::with_capacity(self.len() + 1);
        v.extend_from_slice(self);
        v.push(0);
        v
    }

    fn valid(&self) -> crate::Result<&[u16]> {
        if self.contains(&0xFFFD) {
            crate::bail!("Invalid wide char sequence contains U+FFFD");
        } else {
            Ok(self)
        }
    }

    fn to_os_string(&self) -> OsString {
        OsString::from_wide(self)
    }

    fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.to_os_string())
    }

    #[cfg(all(feature = "text_patch", not(feature = "text_extracting")))]
    fn lookup(&self) -> crate::Result<Vec<u16>> {
        use crate::utils::exts::str_ext::StrExt;
        Ok(self.to_string()?.lookup()?.as_bytes().to_wide_utf8())
    }

    #[cfg(feature = "text_patch")]
    fn lookup_or_add_item(&self) -> crate::Result<Vec<u16>> {
        use crate::utils::exts::str_ext::StrExt;
        Ok(self
            .to_string()?
            .lookup_or_add_item()?
            .as_bytes()
            .to_wide_utf8())
    }

    #[cfg(all(feature = "text_patch", not(feature = "text_extracting")))]
    fn lookup_null(&self) -> crate::Result<Vec<u16>> {
        use crate::utils::exts::str_ext::StrExt;
        Ok(self.to_string()?.lookup()?.as_bytes().to_wide_null_utf8())
    }

    #[cfg(feature = "text_patch")]
    fn lookup_or_add_item_null(&self) -> crate::Result<Vec<u16>> {
        use crate::utils::exts::str_ext::StrExt;
        Ok(self
            .to_string()?
            .lookup_or_add_item()?
            .as_bytes()
            .to_wide_null_utf8())
    }
}
