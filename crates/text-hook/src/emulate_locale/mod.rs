use ntapi::ntnls::NLSTABLEINFO;
use ntapi::ntrtl::RtlInitNlsTables;
use ntapi::ntrtl::RtlResetRtlTranslations;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_READONLY, PAGE_READWRITE, VirtualAlloc, VirtualProtect,
};

use crate::utils::mem::align_up;
use crate::utils::win32::with_wow64_redirection_disabled;
use crate::{debug, print_system_error_message};

pub unsafe fn set_process_nls_tables(
    ansi_file: &str,
    oem_file: &str,
    lang_file: &str,
) -> anyhow::Result<()> {
    let (ansi_buf, oem_buf, lang_buf) = with_wow64_redirection_disabled(|| {
        let sysdir = crate::utils::win32::get_system_directory().unwrap();
        let ansi_path = format!("{}\\{}", sysdir, ansi_file);
        let oem_path = format!("{}\\{}", sysdir, oem_file);
        let lang_path = format!("{}\\{}", sysdir, lang_file);

        let ansi_buf = std::fs::read(&ansi_path)?;
        let oem_buf = std::fs::read(&oem_path)?;
        let lang_buf = std::fs::read(&lang_path)?;

        anyhow::Ok((ansi_buf, oem_buf, lang_buf))
    })?;

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
            print_system_error_message!();
            anyhow::bail!("VirtualAlloc failed");
        }

        let base = mem as *mut u8;
        core::ptr::copy_nonoverlapping(ansi_buf.as_ptr(), base, ansi_buf.len());
        if a_len > ansi_buf.len() {
            core::ptr::write_bytes(base.add(ansi_buf.len()), 0, a_len - ansi_buf.len());
        }
        let oem_base = base.add(a_len);
        core::ptr::copy_nonoverlapping(oem_buf.as_ptr(), oem_base, oem_buf.len());
        if o_len > oem_buf.len() {
            core::ptr::write_bytes(oem_base.add(oem_buf.len()), 0, o_len - oem_buf.len());
        }
        let lang_base = base.add(a_len + o_len);
        core::ptr::copy_nonoverlapping(lang_buf.as_ptr(), lang_base, lang_buf.len());
        if l_len > lang_buf.len() {
            core::ptr::write_bytes(lang_base.add(lang_buf.len()), 0, l_len - lang_buf.len());
        }

        let mut old_prot: u32 = 0;
        if VirtualProtect(mem, total, PAGE_READONLY, &mut old_prot) == 0 {
            print_system_error_message!();
            anyhow::bail!("VirtualProtect failed");
        }

        let mut table_info: NLSTABLEINFO = core::mem::zeroed();
        let ansi_ptr = base as *mut u16;
        let oem_ptr = oem_base as *mut u16;
        let case_ptr = lang_base as *mut u16;

        RtlInitNlsTables(ansi_ptr, oem_ptr, case_ptr, &mut table_info);

        RtlResetRtlTranslations(&mut table_info);

        let Some(peb) = crate::utils::nt::get_current_peb() else {
            anyhow::bail!("get_current_peb fails");
        };

        let peb_ref = &mut *peb;

        peb_ref.AnsiCodePageData = ansi_ptr as *mut _;
        peb_ref.OemCodePageData = oem_ptr as *mut _;
        peb_ref.UnicodeCaseTableData = case_ptr as *mut _
    }

    Ok(())
}

pub fn init_japanese_nls_example() {
    unsafe {
        if let Err(e) = set_process_nls_tables("C_932.NLS", "C_932.NLS", "l_intl.nls") {
            debug!("init nls fails with {e}");
        }

        // 下面是测试代码，不要管
        let s = String::from_utf16_lossy(&crate::code_cvt::ansi_to_wide_char(&[
            0x82, 0xB1, 0x82, 0xF1, 0x82, 0xC9, 0x82, 0xBF, 0x82, 0xCD,
        ]));
        debug!("{s}");
    }
}
