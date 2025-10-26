use std::{borrow::Cow, path::Path};
use translate_macros::ffi_catch_unwind;
use windows_sys::{
    Win32::{
        Foundation::MAX_PATH,
        Globalization::{CP_ACP, WideCharToMultiByte},
        System::WindowsProgramming::{GetPrivateProfileIntA, GetPrivateProfileStringA},
    },
    core::{PCSTR, PSTR},
};

use crate::{debug, utils::mem::slice_until_null};

fn query_game_ini_string(section: &str, key: &str) -> Option<String> {
    match (section, key) {
        ("LUSTS", "CDROM") => Some("Y:\\".to_string()),
        ("LUSTS", "InstDIR") => {
            if let Ok(exe) = std::env::current_exe()
                && let Some(dir) = exe.parent()
            {
                return Some(format!("{}\\", dir.display())); // 注意加上尾部的 "\"
            }
            None
        }
        _ => None,
    }
}

fn query_game_ini_int(section: &str, key: &str) -> Option<i32> {
    match (section, key) {
        ("Games", "InstCount") => Some(1),
        ("LUSTS", "Music") => Some(1),
        ("LUSTS", "Voice") => Some(1),
        ("LUSTS", "VoiceCD") => Some(0),
        ("LUSTS", "Data") => Some(0),
        ("LUSTS", "Verson") => Some(100),
        _ => None,
    }
}

unsafe fn matched_ini(file_name: PCSTR) -> bool {
    let file = unsafe { String::from_utf8_lossy(slice_until_null(file_name, MAX_PATH as _)) };

    Path::new(file.as_ref())
        .file_name()
        .map(|f| f.to_string_lossy().eq_ignore_ascii_case("Assemblage.INI"))
        .unwrap_or(false)
}

unsafe fn to_string(app_name: PCSTR, key_name: PCSTR) -> (String, String) {
    let section = if !app_name.is_null() {
        String::from_utf8_lossy(unsafe { slice_until_null(app_name, MAX_PATH as _) })
    } else {
        Cow::Borrowed("")
    };

    let key = if !key_name.is_null() {
        String::from_utf8_lossy(unsafe { slice_until_null(key_name, MAX_PATH as _) })
    } else {
        Cow::Borrowed("")
    };

    (section.into_owned(), key.into_owned())
}

#[ffi_catch_unwind(0u32)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn get_private_profiles_string(
    lp_app_name: PCSTR,
    lp_key_name: PCSTR,
    lp_default: PCSTR,
    lp_returned_string: PSTR,
    n_size: u32,
    lp_file_name: PCSTR,
) -> u32 {
    unsafe {
        if lp_file_name.is_null() {
            return 0;
        }

        if matched_ini(lp_file_name) {
            let (section, key) = to_string(lp_app_name, lp_key_name);
            debug!("section: {section}, key: {key}");

            if let Some(val) = query_game_ini_string(&section, &key) {
                debug!("found value: {val}");
                // 将UTF-8字符串转换为宽字符字符串（UTF-16）
                let wide_str: Vec<u16> = val.encode_utf16().collect();
                let wide_len = wide_str.len() as i32;

                // 计算所需的ANSI缓冲区大小
                let ansi_size = WideCharToMultiByte(
                    CP_ACP,
                    0,
                    wide_str.as_ptr(),
                    wide_len,
                    core::ptr::null_mut(),
                    0,
                    core::ptr::null(),
                    core::ptr::null_mut(),
                );

                if ansi_size == 0 {
                    return 0; // 转换失败
                }

                // 分配ANSI缓冲区
                let mut ansi_buffer = Vec::<u8>::with_capacity(ansi_size as usize);
                let ansi_ptr = ansi_buffer.as_mut_ptr();

                // 执行实际转换
                let result = WideCharToMultiByte(
                    CP_ACP,
                    0,
                    wide_str.as_ptr(),
                    wide_len,
                    ansi_ptr,
                    ansi_size,
                    core::ptr::null(),
                    core::ptr::null_mut(),
                );

                if result == 0 {
                    return 0; // 转换失败
                }

                // 设置向量长度并确保以null结尾
                ansi_buffer.set_len(ansi_size as usize);

                // 计算实际需要复制的长度
                let copy_len = ansi_buffer.len().min(n_size as usize);

                // 复制到输出缓冲区
                core::ptr::copy_nonoverlapping(ansi_buffer.as_ptr(), lp_returned_string, copy_len);

                // 确保在缓冲区不足时正确终止字符串
                if copy_len < n_size as usize {
                    // 如果空间足够，确保null终止
                    *lp_returned_string.add(copy_len) = 0;
                } else if n_size > 0 {
                    // 如果缓冲区不足，确保在末尾null终止
                    *lp_returned_string.add(n_size as usize - 1) = 0;
                }

                return copy_len.min(n_size as usize).saturating_sub(1) as u32;
            }
        }

        debug!("passed");

        GetPrivateProfileStringA(
            lp_app_name,
            lp_key_name,
            lp_default,
            lp_returned_string,
            n_size,
            lp_file_name,
        )
    }
}

#[ffi_catch_unwind(n_default as _)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn get_private_profiles_int(
    lp_app_name: PCSTR,
    lp_key_name: PCSTR,
    n_default: i32,
    lp_file_name: PCSTR,
) -> u32 {
    unsafe {
        if lp_file_name.is_null() {
            return n_default as _;
        }

        if matched_ini(lp_file_name) {
            let (section, key) = to_string(lp_app_name, lp_key_name);
            debug!("section: {section}, key: {key}");

            if let Some(val) = query_game_ini_int(&section, &key) {
                debug!("found value: {val}");
                return val as _;
            }

            return n_default as _;
        }

        debug!("passed");
        GetPrivateProfileIntA(lp_app_name, lp_key_name, n_default, lp_file_name)
    }
}
