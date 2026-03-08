use translate_macros::detour_trait;

#[detour_trait]
pub trait LifeCycleHook: Send + Sync + 'static {
    #[detour(dll = "kernel32.dll", symbol = "ExitProcess")]
    unsafe fn exit_process(_u_exit_code: u32) {
        #[cfg(not(feature = "attach_clean_up"))]
        unimplemented!();

        #[cfg(feature = "attach_clean_up")]
        {
            crate::hook::impls::attach_clean_up();
            unsafe { crate::call!(HOOK_EXIT_PROCESS, _u_exit_code) };
        }
    }
}
