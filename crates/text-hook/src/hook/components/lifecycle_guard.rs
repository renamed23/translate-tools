use crate::hook::api_hooks::process::{ExitProcess, HOOK_EXIT_PROCESS};

#[allow(dead_code)]
pub struct LifecycleGuard;

cfg_select! {
    feature = "bind_lifecycle_guard" => {
        type LifecycleGuardSlot = crate::hook::impls::HookImplType;
    }
    _ => {
        type LifecycleGuardSlot = LifecycleGuard;
    }
}

impl ExitProcess for LifecycleGuardSlot {
    unsafe fn exit_process(u_exit_code: u32) {
        crate::hook::entry::attach_cleanup();
        unsafe { crate::call!(HOOK_EXIT_PROCESS, u_exit_code) };
    }
}
