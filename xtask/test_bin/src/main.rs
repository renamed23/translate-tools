mod font;
mod vfs;
mod window_title;

use windows_sys::{Win32::Foundation::HMODULE, core::BOOL};

#[link(name = "text-hook")]
unsafe extern "system" {
    fn DllMain(_hinst_dll: HMODULE, fdw_reason: u32, _lpv_reserved: *mut core::ffi::c_void)
    -> BOOL;
}

fn main() -> anyhow::Result<()> {
    println!("text-hook 已安装: {:p}", DllMain as *const ());
    vfs::run_tests()?;
    window_title::run_tests()?;
    font::run_tests()?;
    Ok(())
}
