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

    pub fn debug(s: &str) {
        let wide: Vec<u16> = s.encode_utf16().chain(core::iter::once(0)).collect();
        unsafe {
            OutputDebugStringW(wide.as_ptr());
        }

        CONSOLE_INIT.call_once(|| {
            // 分配控制台窗口
            unsafe {
                AllocConsole();
                SetConsoleCP(CP_UTF8);
                SetConsoleOutputCP(CP_UTF8);
            }
        });

        println!("{s}");
    }

    pub fn get_system_error_message() -> Option<String> {
        unsafe {
            let error_code = GetLastError();
            get_system_error_message_from_ec(error_code)
        }
    }

    pub fn get_system_error_message_from_ntstatus(status: NTSTATUS) -> Option<String> {
        unsafe {
            let error_code = RtlNtStatusToDosError(status);
            get_system_error_message_from_ec(error_code)
        }
    }

    pub fn get_system_error_message_from_ec(ec: u32) -> Option<String> {
        unsafe {
            let mut buffer = [0u16; 512];
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
macro_rules! debug {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "debug_output")]
            {
                $crate::debug_output::debug_impl::debug(&format!(
                    "[{}:{}] {}",
                    file!(),
                    line!(),
                    format_args!($($arg)*)
                ));
            }
            #[cfg(not(feature = "debug_output"))]
            {
            }
        }
    };
}

#[macro_export]
macro_rules! debug_msg {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "debug_output")]
            {
                $crate::debug_output::debug_impl::debug(&format!(
                    "{}",
                    format_args!($($arg)*)
                ));
            }
            #[cfg(not(feature = "debug_output"))]
            {
            }
        }
    };
}

#[macro_export]
macro_rules! print_system_error_message {
    () => {{
        #[cfg(feature = "debug_output")]
        {
            if let Some(msg) = $crate::debug_output::debug_impl::get_system_error_message() {
                $crate::debug!("[system error]: {}", msg);
            }
        }
        #[cfg(not(feature = "debug_output"))]
        {}
    }};
    (EC $ec: expr) => {{
        #[cfg(feature = "debug_output")]
        {
            if let Some(msg) =
                $crate::debug_output::debug_impl::get_system_error_message_from_ec($ec)
            {
                $crate::debug!("[system error]: {}", msg);
            }
        }
        #[cfg(not(feature = "debug_output"))]
        {}
    }};
    (NT $ntstatus: expr) => {{
        #[cfg(feature = "debug_output")]
        {
            if let Some(msg) =
                $crate::debug_output::debug_impl::get_system_error_message_from_ntstatus($ntstatus)
            {
                $crate::debug!("[system error]: {}", msg);
            }
        }
        #[cfg(not(feature = "debug_output"))]
        {}
    }};
}
