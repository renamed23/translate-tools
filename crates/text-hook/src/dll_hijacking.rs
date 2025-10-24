mod dll {
    static mut HMOD: usize = 0;

    static mut ADDR_GET_FILE_VERSION_INFO_A: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_SIZE_A: usize = 0;
    static mut ADDR_VER_QUERY_VALUE_A: usize = 0;
    static mut ADDR_VER_QUERY_VALUE_W: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_EX_W: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_SIZE_EX_W: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_SIZE_W: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_W: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_EX_A: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_SIZE_EX_A: usize = 0;
    static mut ADDR_VER_FIND_FILE_A: usize = 0;
    static mut ADDR_VER_INSTALL_FILE_A: usize = 0;
    static mut ADDR_GET_FILE_VERSION_INFO_BY_HANDLE: usize = 0;
    static mut ADDR_VER_FIND_FILE_W: usize = 0;
    static mut ADDR_VER_INSTALL_FILE_W: usize = 0;
    static mut ADDR_VER_LANGUAGE_NAME_A: usize = 0;
    static mut ADDR_VER_LANGUAGE_NAME_W: usize = 0;

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoA")]
    pub unsafe extern "system" fn lib_get_file_version_info_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoSizeA")]
    pub unsafe extern "system" fn lib_get_file_version_info_size_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_SIZE_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerQueryValueA")]
    pub unsafe extern "system" fn lib_ver_query_value_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_QUERY_VALUE_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerQueryValueW")]
    pub unsafe extern "system" fn lib_ver_query_value_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_QUERY_VALUE_W,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoExW")]
    pub unsafe extern "system" fn lib_get_file_version_info_ex_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_EX_W,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoSizeExW")]
    pub unsafe extern "system" fn lib_get_file_version_info_size_ex_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_SIZE_EX_W,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoSizeW")]
    pub unsafe extern "system" fn lib_get_file_version_info_size_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_SIZE_W,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoW")]
    pub unsafe extern "system" fn lib_get_file_version_info_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_W,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoExA")]
    pub unsafe extern "system" fn lib_get_file_version_info_ex_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_EX_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoSizeExA")]
    pub unsafe extern "system" fn lib_get_file_version_info_size_ex_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_SIZE_EX_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerFindFileA")]
    pub unsafe extern "system" fn lib_ver_find_file_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_FIND_FILE_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerInstallFileA")]
    pub unsafe extern "system" fn lib_ver_install_file_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_INSTALL_FILE_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "GetFileVersionInfoByHandle")]
    pub unsafe extern "system" fn lib_get_file_version_info_by_handle() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_GET_FILE_VERSION_INFO_BY_HANDLE,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerFindFileW")]
    pub unsafe extern "system" fn lib_ver_find_file_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_FIND_FILE_W,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerInstallFileW")]
    pub unsafe extern "system" fn lib_ver_install_file_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_INSTALL_FILE_W,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerLanguageNameA")]
    pub unsafe extern "system" fn lib_ver_language_name_a() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_LANGUAGE_NAME_A,
        );
    }

    #[unsafe(naked)]
    #[unsafe(link_section = ".text")]
    #[unsafe(export_name = "VerLanguageNameW")]
    pub unsafe extern "system" fn lib_ver_language_name_w() {
        ::std::arch::naked_asm!(
            "jmp [{0}]",
            sym ADDR_VER_LANGUAGE_NAME_W,
        );
    }

    #[allow(static_mut_refs)]
    pub(super) unsafe extern "system" fn load_library() {
        unsafe {
            let hmod = crate::hook_utils::load_hijacked_library("version.dll")
                .expect("Could not find version.dll");
            let addrs = crate::hook_utils::get_module_symbol_addrs_from_handle(
                hmod,
                &[
                    c"GetFileVersionInfoA".as_ptr(),
                    c"GetFileVersionInfoSizeA".as_ptr(),
                    c"VerQueryValueA".as_ptr(),
                    c"VerQueryValueW".as_ptr(),
                    c"GetFileVersionInfoExW".as_ptr(),
                    c"GetFileVersionInfoSizeExW".as_ptr(),
                    c"GetFileVersionInfoSizeW".as_ptr(),
                    c"GetFileVersionInfoW".as_ptr(),
                    c"GetFileVersionInfoExA".as_ptr(),
                    c"GetFileVersionInfoSizeExA".as_ptr(),
                    c"VerFindFileA".as_ptr(),
                    c"VerInstallFileA".as_ptr(),
                    c"GetFileVersionInfoByHandle".as_ptr(),
                    c"VerFindFileW".as_ptr(),
                    c"VerInstallFileW".as_ptr(),
                    c"VerLanguageNameA".as_ptr(),
                    c"VerLanguageNameW".as_ptr(),
                ],
            )
            .expect("Could not get symbol addrs for version.dll");

            HMOD = hmod as usize;
            ADDR_GET_FILE_VERSION_INFO_A = addrs[0] as usize;
            ADDR_GET_FILE_VERSION_INFO_SIZE_A = addrs[1] as usize;
            ADDR_VER_QUERY_VALUE_A = addrs[2] as usize;
            ADDR_VER_QUERY_VALUE_W = addrs[3] as usize;
            ADDR_GET_FILE_VERSION_INFO_EX_W = addrs[4] as usize;
            ADDR_GET_FILE_VERSION_INFO_SIZE_EX_W = addrs[5] as usize;
            ADDR_GET_FILE_VERSION_INFO_SIZE_W = addrs[6] as usize;
            ADDR_GET_FILE_VERSION_INFO_W = addrs[7] as usize;
            ADDR_GET_FILE_VERSION_INFO_EX_A = addrs[8] as usize;
            ADDR_GET_FILE_VERSION_INFO_SIZE_EX_A = addrs[9] as usize;
            ADDR_VER_FIND_FILE_A = addrs[10] as usize;
            ADDR_VER_INSTALL_FILE_A = addrs[11] as usize;
            ADDR_GET_FILE_VERSION_INFO_BY_HANDLE = addrs[12] as usize;
            ADDR_VER_FIND_FILE_W = addrs[13] as usize;
            ADDR_VER_INSTALL_FILE_W = addrs[14] as usize;
            ADDR_VER_LANGUAGE_NAME_A = addrs[15] as usize;
            ADDR_VER_LANGUAGE_NAME_W = addrs[16] as usize;
        }
    }

    #[allow(static_mut_refs)]
    pub(super) unsafe extern "system" fn unload_library() {
        unsafe {
            ::winapi::um::libloaderapi::FreeLibrary(HMOD as _);

            HMOD = 0;
            ADDR_GET_FILE_VERSION_INFO_A = 0;
            ADDR_GET_FILE_VERSION_INFO_SIZE_A = 0;
            ADDR_VER_QUERY_VALUE_A = 0;
            ADDR_VER_QUERY_VALUE_W = 0;
            ADDR_GET_FILE_VERSION_INFO_EX_W = 0;
            ADDR_GET_FILE_VERSION_INFO_SIZE_EX_W = 0;
            ADDR_GET_FILE_VERSION_INFO_SIZE_W = 0;
            ADDR_GET_FILE_VERSION_INFO_W = 0;
            ADDR_GET_FILE_VERSION_INFO_EX_A = 0;
            ADDR_GET_FILE_VERSION_INFO_SIZE_EX_A = 0;
            ADDR_VER_FIND_FILE_A = 0;
            ADDR_VER_INSTALL_FILE_A = 0;
            ADDR_GET_FILE_VERSION_INFO_BY_HANDLE = 0;
            ADDR_VER_FIND_FILE_W = 0;
            ADDR_VER_INSTALL_FILE_W = 0;
            ADDR_VER_LANGUAGE_NAME_A = 0;
            ADDR_VER_LANGUAGE_NAME_W = 0;
        };
    }
}

pub unsafe extern "system" fn load_library() {
    unsafe { dll::load_library() };
}

pub unsafe extern "system" fn unload_library() {
    unsafe { dll::unload_library() };
}
