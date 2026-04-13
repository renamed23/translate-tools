use std::{sync::Arc, time::Instant};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::ScreenToClient,
    UI::{
        Controls::WM_MOUSELEAVE,
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{
            ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_CONTROL,
            VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT,
            VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            GetClientRect, GetCursorPos, HTCLIENT, WM_CHAR, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
            WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN,
            WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCHITTEST,
            WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETFOCUS,
            WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDBLCLK, WM_XBUTTONDOWN, WM_XBUTTONUP,
        },
    },
};

use egui::{
    ClippedPrimitive, Context as EguiContext, Event as EguiEvent, FullOutput, Key, Modifiers,
    PointerButton, Pos2, RawInput, Rect as EguiRect, Vec2,
};
use egui_glow::Painter as EguiGlowPainter;

use crate::overlay::{OverlayContext, window::set_overlay_click_through};

/// 单帧 egui 渲染过程中暂存的输出数据。
#[derive(Default)]
pub struct EguiFrameData {
    /// `egui` 一帧结束后产出的完整输出。
    pub full_output: Option<FullOutput>,
    /// 经过 tessellate 后的裁剪图元列表。
    pub clipped_primitives: Vec<ClippedPrimitive>,
    /// 当前窗口的像素缩放比。
    pub pixels_per_point: f32,
    /// 当前 overlay 客户区的像素尺寸。
    pub screen_size_px: [u32; 2],
}

/// 管理 egui 输入、帧生命周期与 glow 绘制器的 overlay 状态。
pub struct EguiOverlayState {
    context: EguiContext,
    glow_painter: EguiGlowPainter,
    raw_input: RawInput,
    frame_data: EguiFrameData,
    start_time: Instant,
    modifiers: Modifiers,
    pointer_pos: Option<Pos2>,
    pressed_pointer_buttons_mask: u8,
    click_through: bool,
}

impl EguiOverlayState {
    /// 基于给定的 OpenGL 上下文创建 egui overlay 状态。
    pub fn new(gl: Arc<glow::Context>) -> crate::Result<Self> {
        let glow_painter = EguiGlowPainter::new(gl, "", None, false)
            .map_err(|e| crate::anyhow!("create egui_glow painter failed: {e}"))?;

        Ok(Self {
            context: EguiContext::default(),
            glow_painter,
            raw_input: RawInput::default(),
            frame_data: EguiFrameData::default(),
            start_time: Instant::now(),
            modifiers: Modifiers::default(),
            pointer_pos: None,
            pressed_pointer_buttons_mask: 0,
            click_through: true,
        })
    }

    /// 取出上一帧缓存的完整输出。
    pub const fn take_full_output(&mut self) -> Option<FullOutput> {
        self.frame_data.full_output.take()
    }

    /// 取出上一帧缓存的裁剪图元列表。
    pub fn take_clipped_primitives(&mut self) -> Vec<ClippedPrimitive> {
        core::mem::take(&mut self.frame_data.clipped_primitives)
    }

    /// 返回当前帧记录的屏幕像素尺寸。
    pub const fn screen_size_px(&self) -> [u32; 2] {
        self.frame_data.screen_size_px
    }

    /// 开始一帧 egui 处理并同步输入状态。
    pub fn begin_frame(&mut self, hwnd: HWND) -> crate::Result<()> {
        let screen_size_px = overlay_client_size(hwnd)?;
        let pixels_per_point = overlay_pixels_per_point(hwnd);

        self.update_pointer_from_cursor(hwnd)?;

        self.frame_data.screen_size_px = screen_size_px;
        self.frame_data.pixels_per_point = pixels_per_point;

        self.raw_input.time = Some(self.start_time.elapsed().as_secs_f64());
        self.raw_input.predicted_dt = 1.0 / 60.0;
        self.raw_input.modifiers = self.modifiers;
        self.raw_input.screen_rect = Some(EguiRect::from_min_size(
            Pos2::ZERO,
            Vec2::new(
                screen_size_px[0] as f32 / pixels_per_point,
                screen_size_px[1] as f32 / pixels_per_point,
            ),
        ));

        self.context
            .begin_pass(core::mem::take(&mut self.raw_input));
        Ok(())
    }

    /// 结束一帧 egui 处理并生成渲染图元。
    pub fn end_frame(&mut self, hwnd: HWND) -> crate::Result<()> {
        let full_output = self.context.end_pass();
        let clipped_primitives = self
            .context
            .tessellate(full_output.shapes.clone(), self.frame_data.pixels_per_point);

        self.frame_data.clipped_primitives = clipped_primitives;
        self.frame_data.full_output = Some(full_output);

        let wants_input = self.context.wants_pointer_input() || self.context.wants_keyboard_input();
        let pointer_over_area = self.context.is_pointer_over_area();
        let click_through = !(wants_input || pointer_over_area);

        if click_through != self.click_through {
            crate::debug!(
                "egui click_through changed: {} -> {}, wants_input={}, pointer_over_area={}, \
                 pointer_pos={:?}",
                self.click_through,
                click_through,
                wants_input,
                pointer_over_area,
                self.pointer_pos
            );
            set_overlay_click_through(hwnd, click_through)?;
            self.click_through = click_through;
        }

        Ok(())
    }

    /// 将当前帧的 egui 图元提交到 glow 绘制器。
    pub fn paint(&mut self) {
        let clipped_primitives = self.take_clipped_primitives();
        let full_output = self.take_full_output().unwrap_or_default();

        self.glow_painter.paint_and_update_textures(
            self.frame_data.screen_size_px,
            self.frame_data.pixels_per_point,
            &clipped_primitives,
            &full_output.textures_delta,
        );
    }

    /// 清空当前渲染目标为指定颜色。
    pub fn clear(&self, clear_color: [f32; 4]) {
        egui_glow::painter::clear(self.glow_painter.gl(), self.screen_size_px(), clear_color);
    }

    /// 执行一整帧 egui：开始帧、构建 UI、结束帧并绘制。
    pub fn run(
        &mut self,
        hwnd: HWND,
        run_ui: impl FnOnce(&EguiContext) -> crate::Result<()>,
    ) -> crate::Result<()> {
        self.begin_frame(hwnd)?;
        run_ui(&self.context)?;
        self.end_frame(hwnd)?;
        self.paint();

        Ok(())
    }

    /// 销毁内部 glow painter 持有的 GPU 资源。
    pub fn destroy(&mut self) {
        self.glow_painter.destroy();
    }

    fn push_event(&mut self, event: EguiEvent) {
        self.raw_input.events.push(event);
    }

    fn update_pointer_position(&mut self, pos: Option<Pos2>) {
        match pos {
            Some(pos) if self.pointer_pos != Some(pos) => {
                self.pointer_pos = Some(pos);
                self.push_event(EguiEvent::PointerMoved(pos));
            }
            Some(_) => {}
            None => {
                if self.pointer_pos.take().is_some() {
                    self.push_event(EguiEvent::PointerGone);
                }
            }
        }
    }

    fn release_all_pointer_buttons(&mut self, hwnd: HWND) {
        let pos = self.pointer_pos.or_else(|| {
            cursor_pos_in_client(hwnd, self.frame_data.pixels_per_point)
                .ok()
                .flatten()
        });

        for (button, mask) in [
            (
                PointerButton::Primary,
                pointer_button_mask(PointerButton::Primary),
            ),
            (
                PointerButton::Secondary,
                pointer_button_mask(PointerButton::Secondary),
            ),
            (
                PointerButton::Middle,
                pointer_button_mask(PointerButton::Middle),
            ),
            (
                PointerButton::Extra1,
                pointer_button_mask(PointerButton::Extra1),
            ),
            (
                PointerButton::Extra2,
                pointer_button_mask(PointerButton::Extra2),
            ),
        ] {
            if self.pressed_pointer_buttons_mask & mask != 0 {
                self.push_event(EguiEvent::PointerButton {
                    pos: pos.unwrap_or(Pos2::ZERO),
                    button,
                    pressed: false,
                    modifiers: self.modifiers,
                });
            }
        }

        self.pressed_pointer_buttons_mask = 0;
    }

    fn on_pointer_button(&mut self, hwnd: HWND, pos: Pos2, button: PointerButton, pressed: bool) {
        self.update_pointer_position(Some(pos));
        update_pointer_capture(
            hwnd,
            &mut self.pressed_pointer_buttons_mask,
            button,
            pressed,
        );
        self.push_event(EguiEvent::PointerButton {
            pos,
            button,
            pressed,
            modifiers: self.modifiers,
        });
    }

    fn clear_input_state(&mut self) {
        self.modifiers = Modifiers::default();
        self.pressed_pointer_buttons_mask = 0;
        self.pointer_pos = None;
    }

    fn update_pointer_from_cursor(&mut self, hwnd: HWND) -> crate::Result<()> {
        let pos = cursor_pos_in_client(hwnd, self.frame_data.pixels_per_point)?;
        self.update_pointer_position(pos);

        Ok(())
    }
}

impl Drop for EguiOverlayState {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// 处理 overlay 窗口收到的消息，并将其转换为 egui 输入事件。
pub fn handle_egui_wnd_proc(
    egui: &mut EguiOverlayState,
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> crate::Result<Option<LRESULT>> {
    match msg {
        WM_NCHITTEST => {
            crate::debug!("egui wnd_proc: WM_NCHITTEST -> HTCLIENT");
            return Ok(Some(HTCLIENT as LRESULT));
        }
        WM_SETCURSOR => {
            crate::debug!("egui wnd_proc: WM_SETCURSOR passthrough");
            return Ok(None);
        }
        WM_MOUSEMOVE => {
            ensure_tracking_mouse_leave(hwnd)?;
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.update_pointer_position(Some(pos));
        }
        WM_MOUSELEAVE => {
            egui.update_pointer_position(None);
        }
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, PointerButton::Primary, true);
            return Ok(Some(0));
        }
        WM_LBUTTONUP => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, PointerButton::Primary, false);
            return Ok(Some(0));
        }
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, PointerButton::Secondary, true);
            return Ok(Some(0));
        }
        WM_RBUTTONUP => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, PointerButton::Secondary, false);
            return Ok(Some(0));
        }
        WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, PointerButton::Middle, true);
            return Ok(Some(0));
        }
        WM_MBUTTONUP => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, PointerButton::Middle, false);
            return Ok(Some(0));
        }
        WM_XBUTTONDOWN | WM_XBUTTONDBLCLK => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, xbutton_to_egui_button(w_param), true);
            return Ok(Some(1));
        }
        WM_XBUTTONUP => {
            let pos = lparam_to_client_pos(l_param, egui.frame_data.pixels_per_point);
            egui.on_pointer_button(hwnd, pos, xbutton_to_egui_button(w_param), false);
            return Ok(Some(1));
        }
        WM_MOUSEWHEEL => {
            if let Some(pos) =
                wheel_lparam_to_client_pos(hwnd, l_param, egui.frame_data.pixels_per_point)?
            {
                egui.update_pointer_position(Some(pos));
            }
            let delta = wheel_delta_from_wparam(w_param) as f32 / 120.0;
            egui.push_event(EguiEvent::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(0.0, delta),
                modifiers: egui.modifiers,
            });
            return Ok(Some(0));
        }
        WM_MOUSEHWHEEL => {
            if let Some(pos) =
                wheel_lparam_to_client_pos(hwnd, l_param, egui.frame_data.pixels_per_point)?
            {
                egui.update_pointer_position(Some(pos));
            }
            let delta = wheel_delta_from_wparam(w_param) as f32 / 120.0;
            egui.push_event(EguiEvent::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(delta, 0.0),
                modifiers: egui.modifiers,
            });
            return Ok(Some(0));
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            update_modifiers(&mut egui.modifiers, w_param as u32, true);
            if let Some(key) = vk_to_egui_key(w_param as u32) {
                egui.push_event(EguiEvent::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: (l_param & (1 << 30)) != 0,
                    modifiers: egui.modifiers,
                });
                return Ok(Some(0));
            }
        }
        WM_KEYUP | WM_SYSKEYUP => {
            update_modifiers(&mut egui.modifiers, w_param as u32, false);
            if let Some(key) = vk_to_egui_key(w_param as u32) {
                egui.push_event(EguiEvent::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui.modifiers,
                });
                return Ok(Some(0));
            }
        }
        WM_CHAR => {
            if let Some(ch) = char::from_u32(w_param as u32)
                && !ch.is_control()
            {
                egui.push_event(EguiEvent::Text(ch.to_string()));
                return Ok(Some(0));
            }
        }
        WM_SETFOCUS => {
            egui.raw_input.focused = true;
        }
        WM_KILLFOCUS => {
            egui.raw_input.focused = false;
            if egui.pressed_pointer_buttons_mask != 0 {
                unsafe { ReleaseCapture() };
                egui.release_all_pointer_buttons(hwnd);
            }
            egui.update_pointer_position(None);
            egui.clear_input_state();
        }
        _ => {}
    }

    Ok(None)
}

fn overlay_client_size(hwnd: HWND) -> crate::Result<[u32; 2]> {
    let mut rect = RECT::default();
    unsafe {
        if GetClientRect(hwnd, &raw mut rect) == 0 {
            crate::print_last_error_message!();
            crate::bail!("GetClientRect failed");
        }
    }

    Ok([
        (rect.right - rect.left).max(1) as u32,
        (rect.bottom - rect.top).max(1) as u32,
    ])
}

fn overlay_pixels_per_point(hwnd: HWND) -> f32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        crate::debug!("GetDpiForWindow returned 0, fallback to 96 DPI");
        return 1.0;
    }

    dpi as f32 / 96.0
}

fn ensure_tracking_mouse_leave(hwnd: HWND) -> crate::Result<()> {
    let mut track = TRACKMOUSEEVENT {
        cbSize: core::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };

    unsafe {
        if TrackMouseEvent(&raw mut track) == 0 {
            crate::print_last_error_message!();
            crate::bail!("TrackMouseEvent failed");
        }
    }

    Ok(())
}

fn cursor_pos_in_client(hwnd: HWND, pixels_per_point: f32) -> crate::Result<Option<Pos2>> {
    let mut screen_pos = POINT::default();
    unsafe {
        if GetCursorPos(&raw mut screen_pos) == 0 {
            crate::print_last_error_message!();
            crate::bail!("GetCursorPos failed");
        }

        if ScreenToClient(hwnd, &raw mut screen_pos) == 0 {
            crate::print_last_error_message!();
            crate::bail!("ScreenToClient failed");
        }
    }

    let [width, height] = overlay_client_size(hwnd)?;
    let x = screen_pos.x;
    let y = screen_pos.y;

    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return Ok(None);
    }

    Ok(Some(pos_px_to_points(
        Pos2::new(x as f32, y as f32),
        pixels_per_point,
    )))
}

fn lparam_to_client_pos(l_param: LPARAM, pixels_per_point: f32) -> Pos2 {
    let x = (l_param as i16) as i32;
    let y = ((l_param >> 16) as i16) as i32;
    pos_px_to_points(Pos2::new(x as f32, y as f32), pixels_per_point)
}

fn wheel_lparam_to_client_pos(
    hwnd: HWND,
    l_param: LPARAM,
    pixels_per_point: f32,
) -> crate::Result<Option<Pos2>> {
    let mut screen_pos = POINT {
        x: (l_param as i16) as i32,
        y: ((l_param >> 16) as i16) as i32,
    };

    unsafe {
        if ScreenToClient(hwnd, &raw mut screen_pos) == 0 {
            crate::print_last_error_message!();
            crate::bail!("ScreenToClient failed while convert wheel cursor pos");
        }
    }

    let [width, height] = overlay_client_size(hwnd)?;
    let x = screen_pos.x;
    let y = screen_pos.y;

    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return Ok(None);
    }

    Ok(Some(pos_px_to_points(
        Pos2::new(x as f32, y as f32),
        pixels_per_point,
    )))
}

fn pos_px_to_points(pos_px: Pos2, pixels_per_point: f32) -> Pos2 {
    let pixels_per_point = pixels_per_point.max(1.0);
    Pos2::new(pos_px.x / pixels_per_point, pos_px.y / pixels_per_point)
}

const fn wheel_delta_from_wparam(w_param: WPARAM) -> i16 {
    ((w_param >> 16) & 0xffff) as i16
}

const fn xbutton_to_egui_button(w_param: WPARAM) -> PointerButton {
    match ((w_param >> 16) & 0xffff) as u16 {
        1 => PointerButton::Extra1,
        _ => PointerButton::Extra2,
    }
}

const fn pointer_button_mask(button: PointerButton) -> u8 {
    match button {
        PointerButton::Primary => 1 << 0,
        PointerButton::Secondary => 1 << 1,
        PointerButton::Middle => 1 << 2,
        PointerButton::Extra1 => 1 << 3,
        PointerButton::Extra2 => 1 << 4,
    }
}

fn update_pointer_capture(hwnd: HWND, buttons_down: &mut u8, button: PointerButton, pressed: bool) {
    let mask = pointer_button_mask(button);
    let had_buttons = *buttons_down != 0;

    if pressed {
        *buttons_down |= mask;
    } else {
        *buttons_down &= !mask;
    }

    let has_buttons = *buttons_down != 0;
    match (had_buttons, has_buttons) {
        (false, true) => {
            unsafe { SetCapture(hwnd) };
        }
        (true, false) => {
            unsafe { ReleaseCapture() };
        }
        _ => {}
    }
}

const fn update_modifiers(modifiers: &mut Modifiers, vk: u32, pressed: bool) {
    match vk as u16 {
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => modifiers.shift = pressed,
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => modifiers.ctrl = pressed,
        VK_MENU | VK_LMENU | VK_RMENU => modifiers.alt = pressed,
        VK_LWIN | VK_RWIN => modifiers.mac_cmd = pressed,
        _ => {}
    }
    modifiers.command = modifiers.ctrl || modifiers.mac_cmd;
}

fn vk_to_egui_key(vk: u32) -> Option<Key> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F12, VK_HOME, VK_INSERT, VK_LEFT,
        VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
    };
    Some(match vk as u16 {
        VK_DOWN => Key::ArrowDown,
        VK_LEFT => Key::ArrowLeft,
        VK_RIGHT => Key::ArrowRight,
        VK_UP => Key::ArrowUp,
        VK_ESCAPE => Key::Escape,
        VK_TAB => Key::Tab,
        VK_BACK => Key::Backspace,
        VK_RETURN => Key::Enter,
        VK_INSERT => Key::Insert,
        VK_DELETE => Key::Delete,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_SPACE => Key::Space,
        // 数字键
        0x30..=0x39 => unsafe {
            core::mem::transmute::<u8, Key>(vk as u8 - 0x30 + Key::Num0 as u8)
        },
        // 字母键 (A-Z)
        0x41..=0x5A => unsafe { core::mem::transmute::<u8, Key>(vk as u8 - 0x41 + Key::A as u8) },
        // F1 - F12
        VK_F1..=VK_F12 => unsafe {
            core::mem::transmute::<u8, Key>(vk as u8 - VK_F1 as u8 + Key::F1 as u8)
        },
        _ => return None,
    })
}

impl OverlayContext {
    /// 运行一次 egui UI 构建流程。
    ///
    /// 该方法会使用当前 overlay 窗口句柄驱动 egui 一帧，并在回调中提供
    /// [`EguiContext`] 供调用方构建界面。
    pub fn run_egui(
        &mut self,
        run_ui: impl FnOnce(&EguiContext) -> crate::Result<()>,
    ) -> crate::Result<()> {
        self.egui.run(*self.overlay, run_ui)
    }
}
