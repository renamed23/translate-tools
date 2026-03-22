use cfg_if::cfg_if;

use crate::hook::traits::life_cycle_hook::{ExitProcess, HOOK_EXIT_PROCESS};

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
        crate::hook::impls::attach_clean_up();
        unsafe { crate::call!(HOOK_EXIT_PROCESS, u_exit_code) };
    }
}
