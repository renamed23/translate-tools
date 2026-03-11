use core::ops::Deref;
use glow::HasContext;
use std::sync::Arc;
use windows_sys::Win32::{
    Foundation::{FreeLibrary, HMODULE, HWND},
    Graphics::{
        Gdi::{HDC, ReleaseDC},
        OpenGL::{HGLRC, wglDeleteContext, wglGetCurrentContext, wglMakeCurrent},
    },
    UI::WindowsAndMessaging::DestroyWindow,
};

use crate::print_last_error_message;

/// 拥有所有权的窗口句柄，销毁时自动调用 DestroyWindow
pub struct OwnedHWND(pub HWND);

impl Drop for OwnedHWND {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() && DestroyWindow(self.0) == 0 {
                print_last_error_message!();
            }
        }
    }
}

impl Deref for OwnedHWND {
    type Target = HWND;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// 拥有所有权的设备上下文 (Device Context)
pub struct OwnedHDC {
    pub hdc: HDC,
    pub hwnd: HWND,
}

impl Drop for OwnedHDC {
    fn drop(&mut self) {
        unsafe {
            if !self.hdc.is_null() && ReleaseDC(self.hwnd, self.hdc) == 0 {
                print_last_error_message!();
            }
        }
    }
}

impl Deref for OwnedHDC {
    type Target = HDC;
    fn deref(&self) -> &Self::Target {
        &self.hdc
    }
}

/// 拥有所有权的 OpenGL 渲染上下文 (Rendering Context)
pub struct OwnedHGLRC(pub HGLRC);

impl Drop for OwnedHGLRC {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                if wglGetCurrentContext() == self.0 {
                    wglMakeCurrent(core::ptr::null_mut(), core::ptr::null_mut());
                }
                if wglDeleteContext(self.0) == 0 {
                    print_last_error_message!();
                }
            }
        }
    }
}

impl Deref for OwnedHGLRC {
    type Target = HGLRC;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// 拥有所有权的模块句柄
pub struct OwnedHMODULE(pub HMODULE);

impl Drop for OwnedHMODULE {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() && FreeLibrary(self.0) == 0 {
                print_last_error_message!();
            }
        }
    }
}

impl Deref for OwnedHMODULE {
    type Target = HMODULE;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// 拥有所有权的 glow::Buffer
pub struct OwnedBuffer {
    pub gl: Arc<glow::Context>,
    pub buffer: glow::Buffer,
}

impl Drop for OwnedBuffer {
    fn drop(&mut self) {
        unsafe { self.gl.delete_buffer(self.buffer) };
    }
}

impl Deref for OwnedBuffer {
    type Target = glow::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

/// 拥有所有权的 glow::VertexArray
pub struct OwnedVertexArray {
    pub gl: Arc<glow::Context>,
    pub vao: glow::VertexArray,
}

impl Drop for OwnedVertexArray {
    fn drop(&mut self) {
        unsafe { self.gl.delete_vertex_array(self.vao) };
    }
}

impl Deref for OwnedVertexArray {
    type Target = glow::VertexArray;
    fn deref(&self) -> &Self::Target {
        &self.vao
    }
}

/// 拥有所有权的 glow::Program
pub struct OwnedProgram {
    pub gl: Arc<glow::Context>,
    pub program: glow::Program,
}

impl Drop for OwnedProgram {
    fn drop(&mut self) {
        unsafe { self.gl.delete_program(self.program) };
    }
}

impl Deref for OwnedProgram {
    type Target = glow::Program;
    fn deref(&self) -> &Self::Target {
        &self.program
    }
}

/// 拥有所有权的 glow::Shader
pub struct OwnedShader {
    pub gl: Arc<glow::Context>,
    pub shader: glow::Shader,
}

impl Drop for OwnedShader {
    fn drop(&mut self) {
        unsafe { self.gl.delete_shader(self.shader) };
    }
}

impl Deref for OwnedShader {
    type Target = glow::Shader;
    fn deref(&self) -> &Self::Target {
        &self.shader
    }
}

/// 拥有所有权的 glow::Texture
pub struct OwnedTexture {
    pub gl: Arc<glow::Context>,
    pub texture: glow::Texture,
}

impl Drop for OwnedTexture {
    fn drop(&mut self) {
        unsafe { self.gl.delete_texture(self.texture) };
    }
}

impl Deref for OwnedTexture {
    type Target = glow::Texture;
    fn deref(&self) -> &Self::Target {
        &self.texture
    }
}
