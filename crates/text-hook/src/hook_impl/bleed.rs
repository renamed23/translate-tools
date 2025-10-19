use std::sync::RwLock;
use winapi::ctypes::c_int;
use winapi::shared::minwindef::BOOL;
use winapi::shared::ntdef::LPCSTR;
use winapi::shared::windef::HDC;

use crate::debug;
use crate::hook::CoreHook;
use crate::hook::text_hook::{HOOK_TEXT_OUT_A, TextHook};

#[derive(Default)]
pub struct BleedHook {
    line_max_x: RwLock<Vec<c_int>>,
}

impl BleedHook {
    fn layout_text(&self, x: c_int, y: c_int) -> (c_int, c_int) {
        const START_X: c_int = 18;
        const START_Y: c_int = 19;
        const MAX_X: c_int = 640 - 35; // 宽度限制
        const LINE_HEIGHT: c_int = 24;

        let mut line_max_x = self.line_max_x.write().unwrap();

        // 如果下一个对话，那么重置
        if x == START_X && y == START_Y {
            line_max_x.clear();
        }

        // 是否是重影
        let is_shadow = ((y - START_Y) % LINE_HEIGHT) != 0;
        let line_y = y + if is_shadow { 1 } else { 0 };

        // 计算当前行号（基于起始y和行高）
        let mut line_idx = ((line_y - START_Y) / LINE_HEIGHT).max(0) as usize;

        // 如果行号超出缓存，扩展缓存
        if line_idx >= line_max_x.len() {
            line_max_x.resize(line_idx + 1, 0);
        }

        // 判断当前x是否超过本行最大值，如果超过，更新最大值
        if x > line_max_x[line_idx] {
            line_max_x[line_idx] = x;
        }

        // 根据缓存计算之前所有行的最大x和行数
        let prev_lines_max_x_sum: c_int = line_max_x
            .iter()
            .take(line_idx)
            .map(|max_x| (max_x - MAX_X).max(0))
            .sum();

        // 映射新坐标x，按照思路计算
        let mut mapped_x = x + prev_lines_max_x_sum;
        let mut mapped_y = y;

        // 超出宽度则循环换行
        while mapped_x > MAX_X {
            mapped_x = mapped_x - MAX_X + START_X;
            mapped_y += LINE_HEIGHT;
            line_idx += 1;
        }

        // 保持重影和实际文字同一行
        if is_shadow && mapped_x + 2 > MAX_X {
            mapped_x = mapped_x - MAX_X + START_X;
            mapped_y += LINE_HEIGHT;
        }

        (mapped_x, mapped_y)
    }
}

impl CoreHook for BleedHook {}

impl TextHook for BleedHook {
    unsafe fn text_out_a(&self, hdc: HDC, x: c_int, y: c_int, lp_string: LPCSTR, c: c_int) -> BOOL {
        if lp_string.is_null() || c <= 0 {
            return 0;
        }

        unsafe {
            let input_slice = core::slice::from_raw_parts(lp_string as *const u8, c as usize);

            let this = self as *const _ as *mut BleedHook;
            let (new_x, new_y) = this.as_mut().unwrap().layout_text(x, y);
            debug!("draw text '{input_slice:?}' at ({new_x}, {new_y}) from ({x}, {y})",);

            HOOK_TEXT_OUT_A.call(
                hdc,
                new_x,
                new_y,
                input_slice.as_ptr() as LPCSTR,
                input_slice.len() as i32,
            )
        }
    }
}
