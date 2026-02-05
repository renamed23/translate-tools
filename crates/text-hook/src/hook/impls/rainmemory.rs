use translate_macros::DefaultHook;
use windows_sys::Win32::Foundation::HMODULE;

use crate::debug;
use crate::hook::traits::CoreHook;

#[derive(Default, DefaultHook)]
pub struct RainmemoryHook;

impl CoreHook for RainmemoryHook {
    fn on_process_attach(_hinst_dll: HMODULE) {
        let Some(handle) = crate::utils::win32::get_module_handle("") else {
            debug!("get_module_handle failed");
            return;
        };

        debug!("patch {handle:p}");

        let module_addr = handle as *mut u8;

        unsafe {
            crate::utils::mem::patch::write_jmp_instruction(
                module_addr.add(0x1F9BD1),
                trampoline as _,
            )
            .unwrap();
        }
    }
}

#[unsafe(naked)]
#[unsafe(link_section = ".text")]
unsafe extern "system" fn trampoline() {
    core::arch::naked_asm!(
        "
        pushad;
        pushfd;
        mov ecx,[esp+0x40];
        mov edx,[esp+0x3C]; 
        mov eax,[esp+0x28];
        mov ebx,[esp+0x24];
        push ecx;
        push edx;
        push eax;
        push ebx;
        call {0};
        popfd;
        popad;
        pop edi; 
        pop esi; 
        pop ebx; 
        mov esp,ebp; 
        pop ebp; 
        ret;
        ",
        sym replace_script,
    );
}

#[translate_macros::ffi_catch_unwind]
pub unsafe extern "system" fn replace_script(ptr: *mut u8, len: usize, ptr2: *mut u8, len2: usize) {
    if !process_buffer(ptr, len) {
        process_buffer(ptr2, len2);
    }
}

fn process_buffer(ptr: *mut u8, len: usize) -> bool {
    unsafe {
        #[cfg(not(feature = "patch_extracting"))]
        return crate::patch::try_patching(ptr, len);

        #[cfg(feature = "patch_extracting")]
        {
            if !crate::utils::mem::quick_memory_check_win32(ptr, len) {
                return false;
            }

            // 是否是从Garbro解包出来的脚本长度，筛选无用数据
            if !crate::patch::is_patch_len(len) {
                return false;
            }

            // 检测是否是有效的CP932文本（会HOOK到非常多的奇怪数据，所以做这么多筛选是必要的）
            let (_, _, not_valid_932_text) =
                encoding_rs::SHIFT_JIS.decode(core::slice::from_raw_parts(ptr, len));

            if not_valid_932_text {
                return false;
            }

            crate::patch::try_extracting(ptr, len)
        }
    }
}
