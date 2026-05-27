use std::{
    collections::HashMap,
    path::Path,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicIsize, Ordering},
    },
};

use windows_sys::{
    Win32::{
        Foundation::{
            ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, FALSE, GetLastError, HANDLE,
            INVALID_HANDLE_VALUE, SetLastError, TRUE,
        },
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            FINDEX_INFO_LEVELS, FINDEX_SEARCH_OPS, FindExInfoStandard, FindExSearchNameMatch,
            WIN32_FIND_DATAA, WIN32_FIND_DATAW,
        },
    },
    core::{BOOL, PCSTR, PCWSTR},
};

use crate::{
    hook::api_hooks::filesystem::{
        CreateFile, FindClose, FindFirstFile, FindFirstFileEx, FindNextFile, HOOK_CREATE_FILE_W,
        HOOK_FIND_CLOSE, HOOK_FIND_FIRST_FILE_A, HOOK_FIND_FIRST_FILE_EX_A,
        HOOK_FIND_FIRST_FILE_EX_W, HOOK_FIND_FIRST_FILE_W, HOOK_FIND_NEXT_FILE_A,
        HOOK_FIND_NEXT_FILE_W,
    },
    utils::exts::{
        path_ext::PathExt,
        ptr_ext::PtrExt,
        slice_ext::{ByteSliceExt, CommonSliceExt, WideSliceExt},
    },
};

#[allow(dead_code)]
pub struct Vfs;

type VfsSlot = cfg_select! {
    feature = "bind_vfs" => crate::hook::impls::HookImplType,
    _ => Vfs
};

impl CreateFile for VfsSlot {
    unsafe fn create_file_a(
        lp_file_name: PCSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        unsafe {
            let wide = lp_file_name.to_slice_until_null_scan().to_wide_null(0);

            Self::create_file_w(
                wide.as_ptr(),
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            )
        }
    }

    unsafe fn create_file_w(
        lp_file_name: PCWSTR,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const SECURITY_ATTRIBUTES,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: HANDLE,
    ) -> HANDLE {
        unsafe {
            let path = lp_file_name.to_slice_until_null_scan().to_path_buf();

            match crate::vfs::try_redirect(&path) {
                Ok(Some(target)) => {
                    return crate::call!(
                        HOOK_CREATE_FILE_W,
                        target.to_wide_null().as_ptr(),
                        dw_desired_access,
                        dw_share_mode,
                        lp_security_attributes,
                        dw_creation_disposition,
                        dw_flags_and_attributes,
                        h_template_file,
                    );
                }
                Err(e) => {
                    crate::debug!("VFS redirect failed for {}: {e:?}", path.to_string_lossy());
                }
                _ => {}
            }

            crate::call!(
                HOOK_CREATE_FILE_W,
                lp_file_name,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            )
        }
    }
}

impl FindFirstFile for VfsSlot {
    unsafe fn find_first_file_a(
        lp_file_name: PCSTR,
        lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> HANDLE {
        unsafe {
            if lp_find_file_data.is_null() {
                return crate::call!(HOOK_FIND_FIRST_FILE_A, lp_file_name, lp_find_file_data);
            }

            let wide = lp_file_name.to_slice_until_null_scan().to_wide(0);
            let path = wide.to_path_buf();

            match collect_snapshot_find_first_ex(
                &path,
                FindExInfoStandard,
                FindExSearchNameMatch,
                core::ptr::null_mut(),
                0,
            ) {
                Ok(Some(snapshot)) => {
                    if let Some((handle, first)) = FIND_SNAPSHOTS.insert(snapshot) {
                        convert_find_data_w_to_a(&first, &mut *lp_find_file_data);
                        return handle;
                    }
                    SetLastError(ERROR_FILE_NOT_FOUND);
                    return INVALID_HANDLE_VALUE;
                }
                Err(e) => {
                    crate::debug!("VFS try_enum failed for {}: {e:?}", path.to_string_lossy());
                    return INVALID_HANDLE_VALUE;
                }
                _ => {}
            }

            crate::call!(HOOK_FIND_FIRST_FILE_A, lp_file_name, lp_find_file_data)
        }
    }

    unsafe fn find_first_file_w(
        lp_file_name: PCWSTR,
        lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> HANDLE {
        unsafe {
            if lp_find_file_data.is_null() {
                return crate::call!(HOOK_FIND_FIRST_FILE_W, lp_file_name, lp_find_file_data);
            }

            let path = lp_file_name.to_slice_until_null_scan().to_path_buf();

            match collect_snapshot_find_first_ex(
                &path,
                FindExInfoStandard,
                FindExSearchNameMatch,
                core::ptr::null_mut(),
                0,
            ) {
                Ok(Some(snapshot)) => {
                    if let Some((handle, first)) = FIND_SNAPSHOTS.insert(snapshot) {
                        *lp_find_file_data = first;
                        return handle;
                    }
                    SetLastError(ERROR_FILE_NOT_FOUND);
                    return INVALID_HANDLE_VALUE;
                }
                Err(e) => {
                    crate::debug!("VFS try_enum failed for {}: {e:?}", path.to_string_lossy());
                    return INVALID_HANDLE_VALUE;
                }
                _ => {}
            }

            crate::call!(HOOK_FIND_FIRST_FILE_W, lp_file_name, lp_find_file_data)
        }
    }
}

impl FindFirstFileEx for VfsSlot {
    unsafe fn find_first_file_ex_a(
        lp_file_name: PCSTR,
        f_info_level_id: FINDEX_INFO_LEVELS,
        lp_find_file_data: *mut core::ffi::c_void,
        f_search_op: FINDEX_SEARCH_OPS,
        lp_search_filter: *mut core::ffi::c_void,
        dw_additional_flags: u32,
    ) -> HANDLE {
        unsafe {
            if lp_find_file_data.is_null() {
                return crate::call!(
                    HOOK_FIND_FIRST_FILE_EX_A,
                    lp_file_name,
                    f_info_level_id,
                    lp_find_file_data,
                    f_search_op,
                    lp_search_filter,
                    dw_additional_flags
                );
            }

            let wide = lp_file_name.to_slice_until_null_scan().to_wide(0);
            let path = wide.to_path_buf();

            match collect_snapshot_find_first_ex(
                &path,
                f_info_level_id,
                f_search_op,
                lp_search_filter,
                dw_additional_flags,
            ) {
                Ok(Some(snapshot)) => {
                    if let Some((handle, first)) = FIND_SNAPSHOTS.insert(snapshot) {
                        let ansi_out = lp_find_file_data.cast::<WIN32_FIND_DATAA>();
                        convert_find_data_w_to_a(&first, &mut *ansi_out);
                        return handle;
                    }
                    SetLastError(ERROR_FILE_NOT_FOUND);
                    return INVALID_HANDLE_VALUE;
                }
                Err(e) => {
                    crate::debug!("VFS try_enum failed for {}: {e:?}", path.to_string_lossy());
                    return INVALID_HANDLE_VALUE;
                }
                _ => {}
            }

            crate::call!(
                HOOK_FIND_FIRST_FILE_EX_A,
                lp_file_name,
                f_info_level_id,
                lp_find_file_data,
                f_search_op,
                lp_search_filter,
                dw_additional_flags
            )
        }
    }

    unsafe fn find_first_file_ex_w(
        lp_file_name: PCWSTR,
        f_info_level_id: FINDEX_INFO_LEVELS,
        lp_find_file_data: *mut core::ffi::c_void,
        f_search_op: FINDEX_SEARCH_OPS,
        lp_search_filter: *mut core::ffi::c_void,
        dw_additional_flags: u32,
    ) -> HANDLE {
        unsafe {
            if lp_find_file_data.is_null() {
                return crate::call!(
                    HOOK_FIND_FIRST_FILE_EX_W,
                    lp_file_name,
                    f_info_level_id,
                    lp_find_file_data,
                    f_search_op,
                    lp_search_filter,
                    dw_additional_flags,
                );
            }

            let path = lp_file_name.to_slice_until_null_scan().to_path_buf();

            match collect_snapshot_find_first_ex(
                &path,
                f_info_level_id,
                f_search_op,
                lp_search_filter,
                dw_additional_flags,
            ) {
                Ok(Some(snapshot)) => {
                    if let Some((handle, first)) = FIND_SNAPSHOTS.insert(snapshot) {
                        let out = lp_find_file_data.cast::<WIN32_FIND_DATAW>();
                        *out = first;

                        return handle;
                    }
                    SetLastError(ERROR_FILE_NOT_FOUND);
                    return INVALID_HANDLE_VALUE;
                }
                Err(e) => {
                    crate::debug!("VFS try_enum failed for {}: {e:?}", path.to_string_lossy());
                    return INVALID_HANDLE_VALUE;
                }
                _ => {}
            }

            crate::call!(
                HOOK_FIND_FIRST_FILE_EX_W,
                lp_file_name,
                f_info_level_id,
                lp_find_file_data,
                f_search_op,
                lp_search_filter,
                dw_additional_flags,
            )
        }
    }
}

impl FindNextFile for VfsSlot {
    unsafe fn find_next_file_a(
        h_find_file: HANDLE,
        lp_find_file_data: *mut WIN32_FIND_DATAA,
    ) -> BOOL {
        unsafe {
            if lp_find_file_data.is_null() {
                return crate::call!(HOOK_FIND_NEXT_FILE_A, h_find_file, lp_find_file_data);
            }

            match FIND_SNAPSHOTS.next_entry(h_find_file) {
                NextEntry::Entry(entry) => {
                    convert_find_data_w_to_a(&entry, &mut *lp_find_file_data);
                    return TRUE;
                }
                NextEntry::NoMoreEntry => {
                    SetLastError(ERROR_NO_MORE_FILES);
                    return FALSE;
                }
                NextEntry::HandleNotFound => {}
            }

            crate::call!(HOOK_FIND_NEXT_FILE_A, h_find_file, lp_find_file_data)
        }
    }

    unsafe fn find_next_file_w(
        h_find_file: HANDLE,
        lp_find_file_data: *mut WIN32_FIND_DATAW,
    ) -> BOOL {
        unsafe {
            if lp_find_file_data.is_null() {
                return crate::call!(HOOK_FIND_NEXT_FILE_W, h_find_file, lp_find_file_data);
            }

            match FIND_SNAPSHOTS.next_entry(h_find_file) {
                NextEntry::Entry(entry) => {
                    *lp_find_file_data = entry;
                    return TRUE;
                }
                NextEntry::NoMoreEntry => {
                    SetLastError(ERROR_NO_MORE_FILES);
                    return FALSE;
                }
                NextEntry::HandleNotFound => {}
            }

            crate::call!(HOOK_FIND_NEXT_FILE_W, h_find_file, lp_find_file_data)
        }
    }
}

impl FindClose for VfsSlot {
    unsafe fn find_close(h_find_file: HANDLE) -> BOOL {
        if FIND_SNAPSHOTS.remove(h_find_file) {
            return TRUE;
        }

        unsafe { crate::call!(HOOK_FIND_CLOSE, h_find_file) }
    }
}

static FIND_SNAPSHOTS: LazyLock<FindSnapshotMap> = LazyLock::new(FindSnapshotMap::new);

struct FindSnapshot {
    entries: Vec<WIN32_FIND_DATAW>,
    cursor: usize,
}

impl FindSnapshot {
    fn new(entries: Vec<WIN32_FIND_DATAW>) -> Self {
        Self { entries, cursor: 1 }
    }

    fn next(&mut self) -> Option<&WIN32_FIND_DATAW> {
        let entry = self.entries.get(self.cursor)?;
        self.cursor += 1;
        Some(entry)
    }
}

struct FindSnapshotMap(Mutex<HashMap<isize, FindSnapshot>>);

#[allow(clippy::large_enum_variant)]
enum NextEntry {
    HandleNotFound,
    NoMoreEntry,
    Entry(WIN32_FIND_DATAW),
}

impl FindSnapshotMap {
    fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    fn insert(&self, snapshot: Vec<WIN32_FIND_DATAW>) -> Option<(HANDLE, WIN32_FIND_DATAW)> {
        // 选择奇数作为伪句柄是安全的
        // 《PDC 2005 - Five Things Every Win32 Developer Should Know - Raymond Chen - 2005/09》
        static NEXT_PSEUDO_HANDLE: AtomicIsize = AtomicIsize::new(1);

        let first = *snapshot.first()?;
        let handle = NEXT_PSEUDO_HANDLE.fetch_add(4, Ordering::Relaxed);

        crate::debug!("Get first entry {} for handle {handle}", unsafe {
            first
                .cFileName
                .as_ptr()
                .to_slice_until_null(first.cFileName.len() - 1)
                .to_string_lossy()
        });

        self.0
            .lock()
            .expect("Lock poisoned")
            .insert(handle, FindSnapshot::new(snapshot));

        Some((handle as HANDLE, first))
    }

    fn next_entry(&self, handle: HANDLE) -> NextEntry {
        let handle = handle as isize;
        let mut map = self.0.lock().expect("Lock poisoned");
        let Some(snapshot) = map.get_mut(&handle) else {
            return NextEntry::HandleNotFound;
        };

        match snapshot.next().copied() {
            Some(entry) => {
                crate::debug!("Get next entry {} for handle {handle}", unsafe {
                    entry
                        .cFileName
                        .as_ptr()
                        .to_slice_until_null(entry.cFileName.len() - 1)
                        .to_string_lossy()
                });
                NextEntry::Entry(entry)
            }
            None => NextEntry::NoMoreEntry,
        }
    }

    fn remove(&self, handle: HANDLE) -> bool {
        let handle = handle as isize;
        self.0
            .lock()
            .expect("Lock poisoned")
            .remove(&handle)
            .is_some()
    }
}

unsafe fn convert_find_data_w_to_a(src: &WIN32_FIND_DATAW, dst: &mut WIN32_FIND_DATAA) {
    let cfile = unsafe {
        src.cFileName
            .as_ptr()
            .to_slice_until_null(src.cFileName.len() - 1)
    };

    let calt = unsafe {
        src.cAlternateFileName
            .as_ptr()
            .to_slice_until_null(src.cAlternateFileName.len() - 1)
    };

    let cfile_a = cfile.to_multi_byte_null(0);
    let calt_a = calt.to_multi_byte_null(0);

    dst.dwFileAttributes = src.dwFileAttributes;
    dst.ftCreationTime = src.ftCreationTime;
    dst.ftLastAccessTime = src.ftLastAccessTime;
    dst.ftLastWriteTime = src.ftLastWriteTime;
    dst.nFileSizeHigh = src.nFileSizeHigh;
    dst.nFileSizeLow = src.nFileSizeLow;
    dst.dwReserved0 = src.dwReserved0;
    dst.dwReserved1 = src.dwReserved1;

    dst.cFileName
        .copy_min_from_slice_with_null(cfile_a.as_i8_slice());

    dst.cAlternateFileName
        .copy_min_from_slice_with_null(calt_a.as_i8_slice());
}

unsafe fn collect_snapshot_find_first_ex(
    path: &Path,
    f_info_level_id: FINDEX_INFO_LEVELS,
    f_search_op: FINDEX_SEARCH_OPS,
    lp_search_filter: *mut core::ffi::c_void,
    dw_additional_flags: u32,
) -> crate::Result<Option<Vec<WIN32_FIND_DATAW>>> {
    unsafe {
        crate::vfs::try_enum(path, |p| {
            let mut results = Vec::new();

            let wide_path = p.to_wide_null();

            let mut first = WIN32_FIND_DATAW::default();

            SetLastError(0);

            let handle = crate::call!(
                HOOK_FIND_FIRST_FILE_EX_W,
                wide_path.as_ptr(),
                f_info_level_id,
                (&raw mut first).cast(),
                f_search_op,
                lp_search_filter,
                dw_additional_flags,
            );

            if handle == INVALID_HANDLE_VALUE {
                if GetLastError() == ERROR_FILE_NOT_FOUND {
                    return Ok(results);
                }

                crate::print_last_error_message!();
                crate::bail!("FindFirstFileExW failed");
            }

            results.push(first);

            scopeguard::defer!(
                crate::call!(HOOK_FIND_CLOSE, handle);
            );

            loop {
                let mut next = WIN32_FIND_DATAW::default();

                SetLastError(0);

                let err = crate::call!(HOOK_FIND_NEXT_FILE_W, handle, &raw mut next);

                if err == FALSE {
                    if GetLastError() == ERROR_NO_MORE_FILES {
                        break;
                    }

                    crate::print_last_error_message!();
                    crate::bail!("FindNextFileW failed");
                }

                results.push(next);
            }

            Ok(results)
        })
    }
}
