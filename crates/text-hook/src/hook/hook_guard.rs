use core::cell::Cell;

/// HOOK防重入守卫，可用于避免递归导致栈溢出。
///
/// 假设有一个HOOK函数 `CreateFileW`
/// - 直接调用 `CreateFileW(...)` 可能进入HOOK逻辑，也可能进入原始函数逻辑，
///   通过该守卫可保证在进入HOOK逻辑后，所有函数调用均进入原始函数逻辑
/// - 调用 `crate::call!(HOOK_CREATE_FILE_W, ...)` 保证绝对进入原始函数逻辑
/// - 调用 `HookImplType::create_file_w(...)` 保证绝对进入HOOK逻辑
///
/// 在所有情况下，都应该使用后两种调用方式，但是如果使用第三方代码，比如标准库，
/// 就没办法显式指定了，所以该守卫就是防止标准库代码间接调用导致又一次进入HOOK逻辑。
pub struct HookGuard {
    _private: (),
}

impl HookGuard {
    thread_local! {
        static IS_HOOKING: Cell<bool> = const { Cell::new(false) };
    }

    /// 尝试进入 Hook 逻辑。
    ///
    /// 只有在非重入状态下才会成功返回 `Some(HookGuard)`，同时将状态标记为正在重入。
    pub fn enter() -> Option<Self> {
        Self::IS_HOOKING.with(|cell| {
            if cell.get() {
                None
            } else {
                cell.set(true);
                Some(Self { _private: () })
            }
        })
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        Self::IS_HOOKING.set(false);
    }
}
