use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, GetWindowTextW, RegisterClassExW,
        SetWindowTextW, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
    },
};

pub fn run_tests() -> anyhow::Result<()> {
    let features = std::env::var("TEXT_HOOK_FEATURES").unwrap_or_default();
    let override_on = features.contains("enable_window_title_override");

    let hwnd = create_hidden_window()?;

    let input = "龠齦齪哈哈";
    let expected = if override_on {
        "游戏窗口"
    } else {
        "两别飞哈哈"
    };

    set_text(hwnd, input)?;
    let actual = get_text(hwnd)?;

    if actual != expected {
        anyhow::bail!("窗口标题不匹配: 输入 '{input}', 期望 '{expected}', 实际 '{actual}'");
    }

    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd) };
    Ok(())
}

unsafe extern "system" fn my_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn create_hidden_window() -> anyhow::Result<HWND> {
    let class_name = encode_wide("test_window_title_class");

    // 必须传一个自己的窗口过程，否则 32位 inline hook 会有问题
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(my_wnd_proc),
        lpszClassName: class_name.as_ptr(),
        ..unsafe { std::mem::zeroed() }
    };

    if unsafe { RegisterClassExW(&wc) } == 0 {
        let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        if err != 1410 {
            // ERROR_CLASS_ALREADY_EXISTS
            anyhow::bail!("RegisterClassExW 失败: {err}");
        }
    }

    unsafe {
        let hmodule = GetModuleHandleW(std::ptr::null());

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            std::ptr::null(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hmodule,
            std::ptr::null(),
        );

        if hwnd.is_null() {
            let err = windows_sys::Win32::Foundation::GetLastError();
            anyhow::bail!("CreateWindowExW 失败: {err}");
        }

        Ok(hwnd)
    }
}

fn set_text(hwnd: HWND, text: &str) -> anyhow::Result<()> {
    let wide = encode_wide(text);
    unsafe {
        let result = SetWindowTextW(hwnd, wide.as_ptr());
        if result == 0 {
            let err = windows_sys::Win32::Foundation::GetLastError();
            anyhow::bail!("SetWindowTextW 失败: {err}");
        }
    }
    Ok(())
}

fn get_text(hwnd: HWND) -> anyhow::Result<String> {
    let mut buf = vec![0u16; 512];
    unsafe {
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len == 0 {
            let err = windows_sys::Win32::Foundation::GetLastError();
            if err != 0 {
                anyhow::bail!("GetWindowTextW 失败: {err}");
            }
        }
        let slice = &buf[..len as usize];
        Ok(String::from_utf16_lossy(slice))
    }
}

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
