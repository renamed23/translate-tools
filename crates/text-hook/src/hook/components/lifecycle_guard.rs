use cfg_if::cfg_if;

use crate::hook::traits::process::{ExitProcess, HOOK_EXIT_PROCESS};

#[allow(dead_code)]
pub struct LifecycleGuard;

cfg_if! {
    if #[cfg(feature = "bind_lifecycle_guard")] {
        type LifecycleGuardSlot = crate::hook::impls::HookImplType;
    } else {
        type LifecycleGuardSlot = LifecycleGuard;
    }
}

impl ExitProcess for LifecycleGuardSlot {
    unsafe fn exit_process(u_exit_code: u32) {
        crate::hook::entry::attach_cleanup();
        unsafe { crate::call!(HOOK_EXIT_PROCESS, u_exit_code) };
    }
}
