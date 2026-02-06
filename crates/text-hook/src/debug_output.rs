#[cfg(feature = "debug_output")]
pub(crate) mod debug_impl {
    use std::sync::Once;
    use windows_sys::Win32::{
        Foundation::{GetLastError, NTSTATUS, RtlNtStatusToDosError},
        Globalization::CP_UTF8,
        System::{
            Console::{AllocConsole, SetConsoleCP, SetConsoleOutputCP},
            Diagnostics::Debug::{
                FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS, FormatMessageW,
                OutputDebugStringW,
            },
        },
    };

    static CONSOLE_INIT: Once = Once::new();

    pub fn debug(args: core::fmt::Arguments) {
        use core::fmt::Write;

        let mut s = String::new();
        let _ = s.write_fmt(args);

        let mut wide = Vec::with_capacity(s.len() + 1);
        wide.extend(s.encode_utf16());
        wide.push(0);

        unsafe {
            OutputDebugStringW(wide.as_ptr());
        }

        CONSOLE_INIT.call_once(|| unsafe {
            if AllocConsole() != 0 {
                SetConsoleCP(CP_UTF8);
                SetConsoleOutputCP(CP_UTF8);
            }
        });

        println!("{s}");
    }

    pub fn get_last_error_message() -> Option<String> {
        unsafe {
            let error_code = GetLastError();
            get_last_error_message_from_ec(error_code)
        }
    }

    pub fn get_last_error_message_from_ntstatus(status: NTSTATUS) -> Option<String> {
        unsafe {
            let error_code = RtlNtStatusToDosError(status);
            get_last_error_message_from_ec(error_code)
        }
    }

    pub fn get_last_error_message_from_ec(ec: u32) -> Option<String> {
        unsafe {
            let mut buffer = [0u16; 1024];
            let result = FormatMessageW(
                FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
                core::ptr::null_mut(),
                ec,
                0, // 使用系统默认语言
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                core::ptr::null_mut(),
            );

            if result > 0 {
                let len = result as usize;
                let wide_slice = &buffer[..len];
                String::from_utf16(wide_slice).ok()
            } else {
                None
            }
        }
    }
}

#[macro_export]
macro_rules! fn_name {
    () => {{
        fn __fn_name_marker() {}
        let name = std::any::type_name_of_val(&__fn_name_marker);
        name.strip_suffix("::__fn_name_marker").unwrap_or(name)
    }};
}

#[macro_export]
macro_rules! debug {
    (raw $($arg:tt)*) => {{
        #[cfg(feature = "debug_output")]
        {
            $crate::debug_output::debug_impl::debug(
                format_args!($($arg)*)
            );
        }
    }};

    ($($arg:tt)*) => {{
        #[cfg(feature = "debug_output")]
        {
            $crate::debug_output::debug_impl::debug(
                format_args!(
                    "[{}:{}] {}",
                    $crate::fn_name!(),
                    line!(),
                    format_args!($($arg)*)
                )
            );
        }
    }};
}

#[macro_export]
macro_rules! print_last_error_message {
    () => {{
        #[cfg(feature = "debug_output")]
        {
            if let Some(msg) = $crate::debug_output::debug_impl::get_last_error_message() {
                $crate::debug!("[system error]: {}", msg);
            }
        }
    }};
    (ec $ec: expr) => {{
        #[cfg(feature = "debug_output")]
        {
            if let Some(msg) = $crate::debug_output::debug_impl::get_last_error_message_from_ec($ec)
            {
                $crate::debug!("[system error]: {}", msg);
            }
        }
    }};
    (nt $ntstatus: expr) => {{
        #[cfg(feature = "debug_output")]
        {
            if let Some(msg) =
                $crate::debug_output::debug_impl::get_last_error_message_from_ntstatus($ntstatus)
            {
                $crate::debug!("[system error]: {}", msg);
            }
        }
    }};
}
