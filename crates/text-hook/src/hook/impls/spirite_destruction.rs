use translate_macros::DefaultHook;
use windows_sys::Win32::Graphics::Gdi::{HDC, TextOutW};
use windows_sys::core::{BOOL, PCSTR};

use crate::code_cvt::TextVec;
use crate::debug;
use crate::hook::traits::text_hook::HOOK_TEXT_OUT_A;
use crate::hook::traits::{CoreHook, TextHook};

#[derive(DefaultHook)]
#[exclude(TextHook)]
pub struct SpiriteDestructionHook;

impl CoreHook for SpiriteDestructionHook {
    fn enable_hooks() {
        unsafe { HOOK_TEXT_OUT_A.enable().unwrap() };
    }

    fn disable_hooks() {
        unsafe { HOOK_TEXT_OUT_A.disable().unwrap() };
    }
}

impl TextHook for SpiriteDestructionHook {
    unsafe fn text_out_a(hdc: HDC, x: i32, y: i32, lp_string: PCSTR, c: i32) -> BOOL {
        unsafe {
            let input_slice = crate::utils::mem::slice_from_raw_parts(lp_string, c as usize);

            let buf = crate::code_cvt::wide_char_to_utf8(
                &crate::code_cvt::multi_byte_to_wide_char(input_slice, 936),
            );

            let s = str::from_utf8_unchecked(&buf);

            debug!("Get text: {s}");

            let u16_buf = process_text(s);

            TextOutW(hdc, x, y, u16_buf.as_ptr(), u16_buf.len() as i32)
        }
    }
}

fn process_text(s: &str) -> TextVec<u16> {
    let (name, msg) = split_name_and_message(s);

    #[cfg(feature = "text_extracting")]
    {
        use translate_utils::text::Item;

        if let Some(name) = name {
            crate::text_patch::add_item(Item::with_name(name, msg));
        } else {
            crate::text_patch::add_item(Item::new(msg));
        }

        s.encode_utf16().collect()
    }

    #[cfg(not(feature = "text_extracting"))]
    {
        let name = name.and_then(crate::text_patch::lookup_name);
        let message = crate::text_patch::lookup_message(msg).unwrap_or(msg);

        if let Some(name) = name {
            format!("【{name}】{message}").encode_utf16().collect()
        } else {
            message.encode_utf16().collect()
        }
    }
}

fn split_name_and_message(s: &str) -> (Option<&str>, &str) {
    if let Some(s) = s.strip_prefix('【')
        && let Some((name, message)) = s.split_once('】')
    {
        return (Some(name), message);
    }

    (None, s)
}
