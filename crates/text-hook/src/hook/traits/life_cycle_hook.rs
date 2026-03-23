use translate_macros::detour_trait;

#[detour_trait]
pub trait ExitProcess {
    #[detour(dll = "kernel32.dll", symbol = "ExitProcess")]
    unsafe fn exit_process(u_exit_code: u32);
}
