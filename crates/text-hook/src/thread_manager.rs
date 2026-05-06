use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use crate::hook::{impls::HookImplType, internal_hooks::ThreadManagerStart};

static SIGNAL: AtomicBool = AtomicBool::new(false);
static HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

/// 请求所有受管线程停止。幂等操作。
pub fn shutdown() {
    SIGNAL.store(true, Ordering::Release);
}

/// 如果已请求停止，则返回 `true`。
#[allow(dead_code)]
pub fn is_shutdown() -> bool {
    SIGNAL.load(Ordering::Acquire)
}

/// 启动线程管理器，根据启用的特性生成组件线程。
///
/// # Safety
/// - 必须在 DLL 附加期间调用，且仅能调用一次。
pub unsafe fn start() -> crate::Result<()> {
    let mut guard = HANDLES.lock().expect("Lock poisoned");
    if !guard.is_empty() {
        return Ok(());
    }

    SIGNAL.store(false, Ordering::Release);
    crate::debug!("Starting thread manager");

    #[cfg(feature = "enable_ui_thread")]
    {
        guard.push(std::thread::spawn(crate::ui_thread::run));
    }

    if let Some(handle) = <HookImplType as ThreadManagerStart>::on_thread_manager_start() {
        guard.push(handle);
    }

    Ok(())
}

/// 通知所有托管线程停止并等待它们退出。
///
/// # Safety
/// - 必须在成功调用 [`start`] 之后调用。
pub unsafe fn stop() -> crate::Result<()> {
    crate::debug!("Stopping thread manager");

    shutdown();

    let mut guard = HANDLES.lock().expect("Lock poisoned");
    let handles = core::mem::take(&mut *guard);
    drop(guard);

    for handle in handles {
        handle
            .join()
            .map_err(|_| crate::anyhow!("A managed thread panicked"))?;
    }

    Ok(())
}
