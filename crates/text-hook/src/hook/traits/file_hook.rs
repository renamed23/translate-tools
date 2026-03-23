use translate_macros::detour_trait;
use windows_sys::{
    Win32::{
        Foundation::HANDLE,
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{WIN32_FIND_DATAA, WIN32_FIND_DATAW},
        System::IO::OVERLAPPED,
    },
    core::{BOOL, PCSTR, PCWSTR},
};

#[detour_trait]
pub trait CreateFile {
    #[detour(
        dll = "kernel32.dll",
        symbol = "CreateFileA",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn create_file_a(
        lp_file_name: PCSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE;

    #[detour(
        dll = "kernel32.dll",
        symbol = "CreateFileW",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn create_file_w(
        lp_file_name: PCWSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE;
}

#[detour_trait]
pub trait ReadFile {
    #[allow(unused_variables)]
    #[detour(
        dll = "kernel32.dll",
        symbol = "ReadFile",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn read_file(
        h_file: HANDLE,
        lp_buffer: *mut u8,
        n_number_of_bytes_to_read: u32,
        lp_number_of_bytes_read: *mut u32,
        lp_overlapped: *mut OVERLAPPED,
    ) -> BOOL;
}

#[detour_trait]
pub trait CloseHandle {
    #[detour(
        dll = "kernel32.dll",
        symbol = "CloseHandle",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn close_handle(h_object: HANDLE) -> BOOL;
}

#[detour_trait]
pub trait FindFirstFile {
    #[detour(
        dll = "kernel32.dll",
        symbol = "FindFirstFileA",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn find_first_file_a(
        lp_file_name: PCSTR,
        lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> HANDLE;

    #[detour(
        dll = "kernel32.dll",
        symbol = "FindFirstFileW",
        fallback = "windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE"
    )]
    unsafe fn find_first_file_w(
        lp_file_name: PCWSTR,
        lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> HANDLE;
}

#[detour_trait]
pub trait FindNextFile {
    #[detour(
        dll = "kernel32.dll",
        symbol = "FindNextFileA",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn find_next_file_a(
        h_find_file: HANDLE,
        lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> BOOL;

    #[detour(
        dll = "kernel32.dll",
        symbol = "FindNextFileW",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn find_next_file_w(
        h_find_file: HANDLE,
        lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> BOOL;
}

#[detour_trait]
pub trait FindClose {
    #[detour(
        dll = "kernel32.dll",
        symbol = "FindClose",
        fallback = "windows_sys::Win32::Foundation::FALSE"
    )]
    unsafe fn find_close(h_find_file: HANDLE) -> BOOL;
}
