// minimal_nls.rs
#![allow(non_snake_case)]
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use std::{fs, mem};

use ntapi::ntnls::NLSTABLEINFO;
use ntapi::ntpsapi::NtQueryInformationProcess;
use ntapi::ntpsapi::PROCESS_BASIC_INFORMATION;
use ntapi::ntrtl::RtlInitNlsTables;
use ntapi::ntrtl::RtlResetRtlTranslations;
use scopeguard::defer;
use windows_sys::Win32::Foundation::NTSTATUS;
use windows_sys::Win32::Storage::FileSystem::{
    Wow64DisableWow64FsRedirection, Wow64EnableWow64FsRedirection,
};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_READONLY, PAGE_READWRITE, VirtualAlloc, VirtualProtect,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::debug;

pub unsafe fn get_peb_via_ntquery() -> Result<*mut ntapi::ntpebteb::PEB, NTSTATUS> {
    let mut pbi: PROCESS_BASIC_INFORMATION = mem::zeroed();
    let status = NtQueryInformationProcess(
        GetCurrentProcess() as *mut _,
        0, // ProcessBasicInformation
        &mut pbi as *mut _ as *mut _,
        size_of::<PROCESS_BASIC_INFORMATION>() as u32,
        ptr::null_mut(),
    );

    if status < 0 {
        Err(status)
    } else {
        Ok(pbi.PebBaseAddress)
    }
}

/// helper: get system directory as Rust String
fn get_system_directory() -> Option<String> {
    unsafe {
        let mut buf: [u16; 260] = [0; 260];
        let n = GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32);
        if n == 0 || n as usize >= buf.len() {
            return None;
        }
        let os = OsString::from_wide(&buf[..n as usize]);
        Some(os.to_string_lossy().into_owned())
    }
}

/// align helper
fn round_up(v: usize, a: usize) -> usize {
    ((v + a - 1) / a) * a
}

/// 将三个 NLS 文件合并到一块内存，并把 PEB 指向那儿，
/// 并调用 RtlInitNlsTables / RtlResetRtlTranslations
///
/// `ansi_file`, `oem_file`, `lang_file` 为文件名（仅文件名，不含目录），
/// 示例会从 SystemDirectory 拼路径打开。
pub unsafe fn set_process_nls_tables(
    ansi_file: &str,
    oem_file: &str,
    lang_file: &str,
) -> Result<(), String> {
    Wow64EnableWow64FsRedirection(false);

    defer! {
        Wow64EnableWow64FsRedirection(true);
    };

    let sysdir = crate::utils::win32::get_system_directory().unwrap();
    // let sysdir = ".";
    let ansi_path = format!("{}\\{}", sysdir, ansi_file);
    let oem_path = format!("{}\\{}", sysdir, oem_file);
    let lang_path = format!("{}\\{}", sysdir, lang_file);

    let ansi_buf = fs::read(&ansi_path).map_err(|e| format!("read {} error: {}", ansi_path, e))?;
    let oem_buf = fs::read(&oem_path).map_err(|e| format!("read {} error: {}", oem_path, e))?;
    let lang_buf = fs::read(&lang_path).map_err(|e| format!("read {} error: {}", lang_path, e))?;

    // align each file to 16 bytes like LE does
    let a_len = round_up(ansi_buf.len(), 16);
    let o_len = round_up(oem_buf.len(), 16);
    let l_len = round_up(lang_buf.len(), 16);
    let total = a_len + o_len + l_len;

    // allocate RW memory for tables
    let mem = VirtualAlloc(
        ptr::null_mut(),
        total,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_READWRITE,
    );

    if mem.is_null() {
        return Err("VirtualAlloc failed".into());
    }

    // copy buffers into allocated region
    let base = mem as *mut u8;
    ptr::copy_nonoverlapping(ansi_buf.as_ptr(), base, ansi_buf.len());
    if a_len > ansi_buf.len() {
        ptr::write_bytes(base.add(ansi_buf.len()), 0, a_len - ansi_buf.len());
    }
    let oem_base = base.add(a_len);
    ptr::copy_nonoverlapping(oem_buf.as_ptr(), oem_base, oem_buf.len());
    if o_len > oem_buf.len() {
        ptr::write_bytes(oem_base.add(oem_buf.len()), 0, o_len - oem_buf.len());
    }
    let lang_base = base.add(a_len + o_len);
    ptr::copy_nonoverlapping(lang_buf.as_ptr(), lang_base, lang_buf.len());
    if l_len > lang_buf.len() {
        ptr::write_bytes(lang_base.add(lang_buf.len()), 0, l_len - lang_buf.len());
    }

    let mut table_info: NLSTABLEINFO = mem::zeroed();

    // Call RtlInitNlsTables with pointers to start of each mapped file.
    // RtlInitNlsTables expects PUSHORT (pointer to 16-bit words). We'll cast raw pointers.
    let ansi_ptr = base as *mut u16;
    let oem_ptr = oem_base as *mut u16;
    let case_ptr = lang_base as *mut u16;

    // Note: NlsTableInfo we pass as null here (LE constructs an NLSTABLEINFO and passes pointer).
    // Passing null may be acceptable on some versions, but for robustness you should supply a real NLSTABLEINFO.
    RtlInitNlsTables(ansi_ptr, oem_ptr, case_ptr, &mut table_info);

    // Reset translations so runtime picks up new tables
    RtlResetRtlTranslations(&mut table_info);

    // make readonly
    let mut old_prot: u32 = 0;
    let ok = VirtualProtect(mem, total, PAGE_READONLY, &mut old_prot);
    if ok == 0 {
        return Err("VirtualProtect failed".into());
    }

    // Now write pointers into the current PEB so user-mode code that reads
    // PEB->AnsiCodePageData / OemCodePageData / UnicodeCaseTableData will see our tables.
    // Get current PEB via NtCurrentPeb (from ntapi crate).

    let peb = get_peb_via_ntquery().unwrap();
    if peb.is_null() {
        return Err("NtCurrentPeb returned null".into());
    }

    // The PEB definition in ntapi::ntpebteb::PEB has fields; we only set those that exist.
    // Note: this is inherently version- and arch-dependent. Test carefully.
    // SAFETY: we cast to a u8 pointer and write at the field addresses via offsets obtained from the PEB struct.
    // Here we try to write via known struct fields if available.
    //
    // If build fails because `PEB` in your ntapi version has a slightly different layout,
    // adjust by using pointer arithmetic based on your headers or use the `winapi` crate PEB.
    //
    // For simplicity we do a raw write by offsetting to the fields via referencing the struct members.
    //
    // WARNING: modifying PEB is dangerous. Make backups if needed.
    #[allow(non_snake_case)]
    {
        // The ntapi PEB has members `AnsiCodePageData`, `OemCodePageData`, `UnicodeCaseTableData` in many versions.
        // We'll try to set them via field access if present.
        // Cast to the PEB type defined by ntapi:
        let peb_ref = &mut *peb;

        // Write pointers (PEB fields are *mut u16)
        // NOTE: the field names below are present in ntapi::ntpebteb::PEB as of common versions.
        // If your crate's PEB struct differs, adapt accordingly.
        peb_ref.AnsiCodePageData = ansi_ptr as *mut _;
        peb_ref.OemCodePageData = oem_ptr as *mut _;
        peb_ref.UnicodeCaseTableData = case_ptr as *mut _;
    }

    debug!("emulate over...");

    let s = String::from_utf16_lossy(&crate::code_cvt::ansi_to_wide_char(&[
        0x82, 0xB1, 0x82, 0xF1, 0x82, 0xC9, 0x82, 0xBF, 0x82, 0xCD,
    ]));
    debug!("{s}");

    // Success
    Ok(())
}

// Example simple entry: set to Japanese tables (cp 932). Filenames may vary on your OS.
pub fn init_japanese_nls_example() {
    unsafe {
        if let Err(e) = set_process_nls_tables("C_932.NLS", "C_932.NLS", "l_intl.nls") {
            // log error somehow (your DLL likely has logging)
            // e.g., OutputDebugStringA or your logger
            // For demo, we ignore

            debug!("init nls fails with {e}");
            let _ = e;
        }
    }
}
