/// 简化的错误类型
///
/// 这是一个零大小的错误类型（ZST），只表示有错误发生，
///
/// # 注意
/// 所有转换都丢失了原始错误信息，只保留错误发生的记录。
#[derive(Copy, Clone, Debug)]
pub struct Error;

/// 使用简化的错误类型的结果别名
pub type Result<T> = core::result::Result<T, Error>;

/// 返回错误的宏（带可选调试信息）
///
/// 这个宏用于在遇到错误时提前返回`Err(Error)`。
/// 可选地，可以传递格式字符串和参数，这些信息将被记录到调试输出中。
///
/// # 用法
/// ```
/// // 不带调试信息
/// if condition_fails {
///     bail!();
/// }
///
/// // 带调试信息
/// if let Err(e) = some_operation() {
///     bail!("操作失败: {}", e);
/// }
/// ```
#[macro_export]
macro_rules! bail {
    () => {
        return Err($crate::Error);
    };
    ($($arg:tt)*) => {
        {
            $crate::debug!($($arg)*);
            return Err($crate::Error);
        }
    };
}

/// 创建错误的宏（带可选调试信息）
///
/// 这个宏用于直接创建一个`Error`实例。
/// 可选地，可以传递格式字符串和参数，这些信息将被记录到调试输出中。
///
/// # 用法
/// ```
/// // 不带调试信息
/// let err = anyhow!();
///
/// // 带调试信息
/// let err = anyhow!("文件未找到: {}", filename);
/// ```
#[macro_export]
macro_rules! anyhow {
    () => {
        $crate::Error
    };
    ($($arg:tt)*) => {
        {
            $crate::debug!($($arg)*);
            $crate::Error
        }
    };
}

impl<E: std::error::Error> From<E> for Error {
    fn from(e: E) -> Self {
        crate::debug!(raw "error: {e}");
        Error
    }
}
