use windows_sys::Win32::Globalization::SetThreadLocale;

pub fn set_japanese_locale() {
    unsafe { SetThreadLocale(0x0411) };
}
