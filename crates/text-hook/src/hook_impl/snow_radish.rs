use once_cell::sync::OnceCell;
use std::ffi::CStr;
use std::mem;
use translate_macros::ffi_catch_unwind;
use winapi::shared::minwindef::{FARPROC, HMODULE};
use winapi::shared::ntdef::LPCSTR;
use winapi::um::libloaderapi::GetProcAddress as WinGetProcAddress;

use crate::debug;
use crate::hook::{Hook, set_hook_instance};
use crate::hook_utils::iat_patch::patch_iat;
use crate::panic_utils::set_debug_panic_hook;

/// ----- DecodeLz function type and storage -----
/// 使用 stdcall（x86）并按你给出的参数顺序：dst_ptr, dst_len, src_ptr, src_len
pub type DecodeLzFn = unsafe extern "system" fn(
    dst_ptr: *mut u8,
    dst_len: usize,
    src_ptr: *const u8,
    src_len: usize,
) -> usize;

/// 保存原始 DecodeLz 地址（第一次发现时写入）
pub static ORIGINAL_DECODE_LZ: OnceCell<DecodeLzFn> = OnceCell::new();

/// 一个很简单的 Hook 实现，使用 Hook trait 的默认方法。
/// 你的实际逻辑（Shift-JIS 映射）在 map_shift_jis 模块里，Hook trait 的默认方法会调用它。
pub struct SnowRadishHook;

#[ffi_catch_unwind(0)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn decode_lz(
    dst_ptr: *mut u8,
    dst_len: usize,
    src_ptr: *const u8,
    src_len: usize,
) -> usize {
    unsafe {
        let orig = ORIGINAL_DECODE_LZ
            .get()
            .expect("ORIGINAL_DECODE_LZ not set");
        let result = orig(dst_ptr, dst_len, src_ptr, src_len);

        crate::patch::try_patching(dst_ptr, dst_len);
        result
    }
}

impl Hook for SnowRadishHook {
    unsafe fn get_proc_address(&self, hmod: HMODULE, proc_name: LPCSTR) -> FARPROC {
        unsafe {
            // NULL 或 ordinal -> 直接转发
            if proc_name.is_null() {
                return WinGetProcAddress(hmod, proc_name);
            }
            let ptr_val = proc_name as usize;
            if ptr_val <= 0xFFFF {
                // ordinal
                return WinGetProcAddress(hmod, proc_name);
            }

            // 尝试读取函数名
            let name = match CStr::from_ptr(proc_name).to_str() {
                Ok(s) => s,
                Err(_) => return WinGetProcAddress(hmod, proc_name),
            };

            // 小写匹配也行，但这里用大小写不敏感比较
            if name.eq_ignore_ascii_case("DecodeLz") {
                debug!("Hook::get_proc_address - Intercepting DecodeLz");

                // 取得真实地址（由 kernel32 提供的 GetProcAddress）
                let real = WinGetProcAddress(hmod, proc_name);
                if !real.is_null() {
                    // 保存原始 DecodeLz 地址（如果尚未保存）
                    // 转换为 DecodeLzFn 并尝试 set
                    let decoded: DecodeLzFn = mem::transmute(real);
                    match ORIGINAL_DECODE_LZ.set(decoded) {
                        Ok(()) => debug!("ORIGINAL_DECODE_LZ saved"),
                        Err(_) => debug!("ORIGINAL_DECODE_LZ was already set"),
                    }

                    // 返回我们自己的实现地址（hook_exports::hooked_decode_lz）
                    // 直接把函数指针转为 FARPROC 返回
                    return mem::transmute::<*const (), FARPROC>(decode_lz as *const ());
                } else {
                    debug!("WinGetProcAddress returned NULL for DecodeLz");
                    return real;
                }
            }

            // 其余函数正常转发
            WinGetProcAddress(hmod, proc_name)
        }
    }
}

/// 初始化：打 IAT 补丁并设置全局 Hook 实例
#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn init() {
    debug!("init_thread start");
    set_debug_panic_hook();

    match unsafe {
        patch_iat(
            "",
            "gdi32.dll",
            &[(
                c"CreateFontIndirectA".as_ptr(),
                crate::hook::create_font_indirect as usize,
            )],
        )
    } {
        Ok(()) => debug!("patch_iat OK"),
        Err(e) => debug!("patch_iat failed with {e}"),
    }

    match unsafe {
        patch_iat(
            "",
            "kernel32.dll",
            &[(
                c"GetProcAddress".as_ptr(),
                crate::hook::get_proc_address as usize,
            )],
        )
    } {
        Ok(()) => debug!("patch_iat OK"),
        Err(e) => debug!("patch_iat failed with {e}"),
    }

    // 设置全局 Hook 实例（如果你之后想换实现，只调用一次 set_hook_instance）
    set_hook_instance(SnowRadishHook);

    debug!("hook instance set");
}
