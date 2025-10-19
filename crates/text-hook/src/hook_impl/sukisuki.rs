use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ffi::CStr,
};
use translate_macros::ffi_catch_unwind;
use winapi::{
    shared::{
        minwindef::DWORD,
        windef::{HDC, HFONT, POINT},
    },
    um::wingdi::{
        CLEARTYPE_QUALITY, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, FIXED_PITCH, FW_NORMAL,
        GetViewportOrgEx, OUT_TT_PRECIS, SelectObject, SetBkMode, SetTextColor, SetViewportOrgEx,
        TRANSPARENT, TextOutW,
    },
};

use crate::{constant, debug};

#[derive(Default)]
struct Layouter {
    // 记录上一个字符的原始位置（由游戏传进来的 x,y）
    prev_pos: Option<(i32, i32)>,
    // 存储遇到的连续全角空格的位置（按遇到顺序），用 VecDeque 以便高效 pop_front/push_back
    stored_space_positions: VecDeque<(i32, i32)>,
}

impl Layouter {
    fn try_layout(&mut self, x: i32, y: i32, text: &str) -> (i32, i32) {
        // 常量与注释中保持一致
        const START_X: i32 = 20;
        const END_X: i32 = 572;
        const LINE_HEIGHT: i32 = 30;
        const CHAR_ADV: i32 = 24;

        // 仅关心第一个字符（按你注释，text 总是单个双字节字符）
        let ch = text.chars().next().unwrap_or('\0');

        // 默认绘制位置就是原位置
        let mut mapped = (x, y);

        // 判断是否与上一个字符“连续”
        let continuous = if let Some((px, py)) = self.prev_pos {
            // 同行且 x 正好相差一个字符宽
            if x == px + CHAR_ADV && y == py {
                true
            // 换行时，前一个在行尾且当前在下一行行首（特殊连续情况）
            } else {
                px == END_X && x == START_X && y == py + LINE_HEIGHT
            }
        } else {
            // 没有前一字符（首字符）视为不连续
            false
        };

        if !continuous {
            // 一旦不连续，清空记录（注释里要求）
            self.stored_space_positions.clear();

            // 如果是全角空格则记录位置，否则什么也不做
            if ch == '\u{3000}' {
                self.stored_space_positions.push_back((x, y));
            }

            // 更新 prev_pos 并返回原位置
            self.prev_pos = Some((x, y));
            return mapped;
        }

        // 到这里说明是连续的
        if ch == '\u{3000}' {
            // 连续的全角空格，记录位置（等待后续字符来“占位”）
            self.stored_space_positions.push_back((x, y));
            mapped = (x, y);
        } else {
            // 连续的非空格字符
            if let Some(first_pos) = self.stored_space_positions.pop_front() {
                // 把当前字符放到第一个记录的位置（并移除该记录）
                mapped = first_pos;
                // 然后把当前字符原来的位置加入到队列尾部（因为该字符被移动，原位可以作为后续被占用的空位）
                self.stored_space_positions.push_back((x, y));
            } else {
                // 否则不做移动，使用原位置
                mapped = (x, y);
            }
        }

        // 更新 prev_pos 为原始位置（用于后续连续性判断）
        self.prev_pos = Some((x, y));
        mapped
    }
}

const TARGET_PX: i32 = 24;
const ENABLE_SHADOW: bool = true;
const SHADOW_OFFSET_X: i32 = 2;
const SHADOW_OFFSET_Y: i32 = 2;
const FONT_QUALITY: DWORD = CLEARTYPE_QUALITY;

thread_local! {
    static CACHED_FONT: Cell<Option<HFONT>> = const { Cell::new(None) };
    static LAYOUTER: RefCell<Layouter> = RefCell::new(Layouter::default());
}

unsafe fn ensure_font() -> HFONT {
    unsafe {
        if let Some(hf) = CACHED_FONT.get() {
            return hf;
        }

        debug!("Creating Font...");

        let face_u16 = crate::utils::u16_with_null(constant::FONT_FACE);
        let hf = CreateFontW(
            -TARGET_PX,
            0,
            0,
            0,
            FW_NORMAL,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_TT_PRECIS,
            0,
            FONT_QUALITY,
            DEFAULT_PITCH | FIXED_PITCH,
            face_u16.as_ptr(),
        );

        CACHED_FONT.set(Some(hf));
        hf
    }
}

unsafe fn styled(hdc: HDC, render_fn: impl Fn()) {
    unsafe {
        let hf = ensure_font();

        let old_obj = SelectObject(hdc, hf as _);
        let old_bkmode = SetBkMode(hdc, TRANSPARENT as i32);
        let white: DWORD = 0x00FFFFFF;
        let old_color = SetTextColor(hdc, white);

        let mut old_pt: POINT = core::mem::zeroed();
        GetViewportOrgEx(hdc, &mut old_pt as *mut POINT);

        if ENABLE_SHADOW {
            let shadow_col: DWORD = 0x00000000;
            SetTextColor(hdc, shadow_col);
            SetViewportOrgEx(
                hdc,
                old_pt.x + SHADOW_OFFSET_X,
                old_pt.y + SHADOW_OFFSET_Y,
                core::ptr::null_mut(),
            );
            render_fn();
            SetViewportOrgEx(hdc, old_pt.x, old_pt.y, core::ptr::null_mut());
            SetTextColor(hdc, white);
        }

        render_fn();

        SetTextColor(hdc, old_color);
        SetBkMode(hdc, old_bkmode);
        SelectObject(hdc, old_obj);
    }
}

// sukisuki
// const HDC_ADDR: usize = 0x45DF40;

// maid
const HDC_ADDR: usize = 0x4750A0;

#[ffi_catch_unwind]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn render_text(x: i32, y: i32, text: *const i8) {
    if text.is_null() {
        debug!("Error: text is null");
        return;
    }

    unsafe {
        let hdc = core::ptr::read(HDC_ADDR as *const HDC);
        if hdc.is_null() {
            debug!("Error: hdc is null");
            return;
        }

        let text = CStr::from_ptr(text);

        let mut buffer = [0u16; 256];
        let written_count = crate::mapping::map_chars(text.to_bytes(), &mut buffer);
        let result = &buffer[..written_count];

        let text = String::from_utf16(result).expect("Invalid utf16 String");

        let (mapped_x, mapped_y) =
            LAYOUTER.with(|layout| layout.borrow_mut().try_layout(x, y, &text));

        debug!("draw text '{text}' at ({mapped_x}, {mapped_y}) from ({x}, {y})");

        styled(hdc, || {
            TextOutW(
                hdc,
                mapped_x,
                mapped_y,
                result.as_ptr(),
                result.len() as i32,
            );
        });
    }
}
