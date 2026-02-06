use ntapi::ntnls::NLSTABLEINFO;
use ntapi::ntrtl::RtlInitNlsTables;
use ntapi::ntrtl::RtlResetRtlTranslations;
use scopeguard::defer;
use windows_sys::Win32::Foundation::MAX_PATH;
use windows_sys::Win32::Globalization::SetThreadLocale;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_READONLY, PAGE_READWRITE, VirtualAlloc, VirtualProtect,
};
use windows_sys::Win32::System::Registry::HKEY;
use windows_sys::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
};
use windows_sys::w;

use crate::utils::mem::align_up;
use crate::utils::mem::slice_until_null;
use crate::utils::win32::with_wow64_redirection_disabled;
use crate::{debug, print_last_error_message};

unsafe fn set_process_nls_tables(
    ansi_file: &str,
    oem_file: &str,
    lang_file: &str,
) -> crate::Result<()> {
    let (ansi_buf, oem_buf, lang_buf) = with_wow64_redirection_disabled(|| {
        let sysdir = crate::utils::win32::get_system_directory()?;
        Ok::<_, crate::Error>((
            std::fs::read(format!("{}\\{}", sysdir, ansi_file))?,
            std::fs::read(format!("{}\\{}", sysdir, oem_file))?,
            std::fs::read(format!("{}\\{}", sysdir, lang_file))?,
        ))
    })?;

    // 对齐内存到16字节边界
    let a_len = align_up(ansi_buf.len(), 16);
    let o_len = align_up(oem_buf.len(), 16);
    let l_len = align_up(lang_buf.len(), 16);
    let total = a_len + o_len + l_len;

    unsafe {
        let mem = VirtualAlloc(
            core::ptr::null_mut(),
            total,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if mem.is_null() {
            print_last_error_message!();
            crate::bail!("VirtualAlloc failed");
        }

        let base = mem as *mut u8;
        let buffers = [(&ansi_buf, a_len), (&oem_buf, o_len), (&lang_buf, l_len)];
        let mut offset = 0;

        for (buffer, aligned_len) in buffers {
            let dest = base.add(offset);
            core::ptr::copy_nonoverlapping(buffer.as_ptr(), dest, buffer.len());
            if aligned_len > buffer.len() {
                core::ptr::write_bytes(dest.add(buffer.len()), 0, aligned_len - buffer.len());
            }

            offset += aligned_len;
        }

        // 将内存保护改为只读
        let mut old_prot: u32 = 0;
        if VirtualProtect(mem, total, PAGE_READONLY, &mut old_prot) == 0 {
            print_last_error_message!();
            crate::bail!("VirtualProtect failed");
        }

        let ansi_ptr = base as *mut u16;
        let oem_ptr = base.add(a_len) as *mut u16;
        let case_ptr = base.add(a_len + o_len) as *mut u16;

        let mut table_info: NLSTABLEINFO = core::mem::zeroed();

        // 初始化NLS表
        RtlInitNlsTables(ansi_ptr, oem_ptr, case_ptr, &mut table_info);

        // 重置运行时翻译表
        RtlResetRtlTranslations(&mut table_info);

        // 获取当前进程的PEB并更新代码页数据指针
        let peb = crate::utils::nt::get_current_peb()?;

        let peb_ref = &mut *peb;

        peb_ref.AnsiCodePageData = ansi_ptr as *mut _;
        peb_ref.OemCodePageData = oem_ptr as *mut _;
        peb_ref.UnicodeCaseTableData = case_ptr as *mut _
    }

    Ok(())
}

fn get_nls_filename_from_registry() -> crate::Result<String> {
    let mut hkey: HKEY = core::ptr::null_mut();

    let result = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            w!("SYSTEM\\CurrentControlSet\\Control\\Nls\\CodePage"),
            0,
            KEY_READ,
            &mut hkey,
        )
    };

    if result != 0 {
        print_last_error_message!();
        crate::bail!("Failed to open registry key");
    }

    defer!(unsafe {
        RegCloseKey(hkey);
    });

    let mut data_type: u32 = 0;
    let mut data: [u16; MAX_PATH as _] = [0; MAX_PATH as _];
    let mut data_size = (data.len() * size_of::<u16>()) as u32;

    let query_result = unsafe {
        RegQueryValueExW(
            hkey,
            w!("932"),
            core::ptr::null_mut(),
            &mut data_type,
            data.as_mut_ptr() as *mut u8,
            &mut data_size,
        )
    };

    if query_result != 0 {
        print_last_error_message!();
        crate::bail!("Failed to query registry value");
    }

    if data_type != REG_SZ {
        crate::bail!("Registry value is not a string type");
    }

    Ok(String::from_utf16_lossy(unsafe {
        slice_until_null(data.as_ptr(), data.len())
    }))
}

#[allow(unused_variables)]
pub fn set_japanese_locale() {
    // 目前还未完全支持对日语区域的模拟，所以这个对某些游戏是必须的
    unsafe { SetThreadLocale(0x0411) };

    let ansi_file = match get_nls_filename_from_registry() {
        Ok(filename) => filename,
        Err(_) => {
            debug!("Failed to get NLS filename from registry, using default");
            "C_932.NLS".to_string()
        }
    };

    unsafe {
        if set_process_nls_tables(&ansi_file, &ansi_file, "l_intl.nls").is_err() {
            debug!("Init nls failed");
        }
    }
}
