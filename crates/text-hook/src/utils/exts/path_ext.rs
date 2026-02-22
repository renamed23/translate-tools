use std::{os::windows::ffi::OsStrExt, path::Path};

pub trait PathExt {
    /// 将路径转换为 UTF-16 编码的 `u16` 向量。
    fn to_wide(&self) -> Vec<u16>;

    /// 将路径转换为以空字符（null-terminated）结尾的 UTF-16 向量。
    /// 常用于调用 Windows FFI API。
    fn to_wide_null(&self) -> Vec<u16>;
}

impl PathExt for Path {
    fn to_wide(&self) -> Vec<u16> {
        self.as_os_str().encode_wide().collect()
    }

    fn to_wide_null(&self) -> Vec<u16> {
        self.as_os_str()
            .encode_wide()
            .chain(core::iter::once(0))
            .collect()
    }
}
