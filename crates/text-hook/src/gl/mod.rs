pub(crate) mod painter;

use glow::HasContext;
use std::sync::Arc;
use windows_sys::{
    Win32::{
        Foundation::HWND,
        Graphics::{
            Gdi::{GetDC, HDC},
            OpenGL::{
                ChoosePixelFormat, GetPixelFormat, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW,
                PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
                SetPixelFormat, wglCreateContext, wglGetProcAddress, wglMakeCurrent,
            },
        },
    },
    s, w,
};

use crate::{
    print_last_error_message,
    utils::{
        exts::slice_ext::ByteSliceExt,
        raii_wrapper::{
            OwnedBuffer, OwnedHDC, OwnedHGLRC, OwnedProgram, OwnedShader, OwnedTexture,
            OwnedVertexArray,
        },
    },
};

/// Windows WGL OpenGL上下文封装
///
/// `GLContext`表示一个完整的OpenGL渲染环境，绑定到一个指定的窗口。
/// 该结构负责管理WGL上下文、窗口设备上下文以及`glow`函数加载器，
/// 并在生命周期结束时自动释放所有相关资源。
///
/// # 生命周期管理
///
/// `GLContext`实现了RAII资源管理：
///
/// - 创建时：
///   - 设置窗口像素格式
///   - 创建并激活OpenGL上下文
///   - 加载所有OpenGL函数
///
/// - 销毁时 (`Drop`)：
///   - 如果当前线程仍绑定该上下文，则先解绑
///   - 删除OpenGL上下文 (`wglDeleteContext`)
///   - 释放窗口设备上下文 (`ReleaseDC`)
///
/// 因此调用者无需手动管理这些底层资源。
///
/// # 线程模型
///
/// OpenGL上下文是**线程局部绑定**的：
///
/// - 一个上下文在任意时刻只能绑定到一个线程
/// - 在其他线程使用该上下文前必须调用`wglMakeCurrent`
///
/// 本结构不会自动在多线程间迁移上下文绑定。
pub struct GLContext {
    pub hglrc: OwnedHGLRC,
    pub hdc: OwnedHDC,
    pub gl: Arc<glow::Context>,
}

impl GLContext {
    /// 创建并初始化一个OpenGL 3.3 Core Profile上下文
    ///
    /// 该函数完成以下步骤：
    ///
    /// 1. 为指定窗口创建WGL OpenGL上下文（内部使用Dummy Context技术）
    /// 2. 激活该上下文到当前线程
    /// 3. 初始化`glow::Context`并加载所有OpenGL函数指针
    /// 4. 构造`GLContext`对象用于后续渲染
    ///
    /// 创建成功后：
    ///
    /// - OpenGL上下文已绑定到当前线程
    /// - 所有OpenGL函数已可通过`glow`调用
    /// - 调用者可以立即执行渲染初始化（如创建shader、VAO等）
    ///
    /// # Safety
    ///
    /// 调用者必须保证：
    ///
    /// - `hwnd`为有效窗口句柄
    /// - 窗口尚未销毁
    /// - 调用线程允许创建并绑定OpenGL上下文
    ///
    /// 同时需注意：
    ///
    /// - OpenGL上下文绑定是**线程局部的**
    /// - 在其他线程使用该上下文前必须重新调用`wglMakeCurrent`
    pub unsafe fn new(hwnd: HWND) -> crate::Result<Self> {
        unsafe {
            let (hdc, hglrc) = create_gl_context(hwnd)?;
            let gl = Arc::new(create_glow_context()?);

            Ok(Self { hdc, hglrc, gl })
        }
    }
}

/// 为传入 hdc 设置像素格式
unsafe fn set_pixel_format(hdc: HDC) -> crate::Result<()> {
    let pfd = PIXELFORMATDESCRIPTOR {
        nSize: core::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
        nVersion: 1,
        dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
        iPixelType: PFD_TYPE_RGBA,
        cColorBits: 32,
        cAlphaBits: 8,
        cDepthBits: 24,
        cStencilBits: 8,
        iLayerType: PFD_MAIN_PLANE as u8,
        ..PIXELFORMATDESCRIPTOR::default()
    };

    unsafe {
        let pf = ChoosePixelFormat(hdc, &pfd);
        if pf == 0 {
            print_last_error_message!();
            crate::bail!("ChoosePixelFormat failed");
        }

        if SetPixelFormat(hdc, pf, &pfd) == 0 {
            print_last_error_message!();
            crate::bail!("SetPixelFormat failed");
        }
    }

    Ok(())
}

/// 为指定窗口创建现代OpenGL 3.3 Core Profile上下文
unsafe fn create_gl_context(hwnd: HWND) -> crate::Result<(OwnedHDC, OwnedHGLRC)> {
    unsafe {
        let hdc_raw = GetDC(hwnd);
        if hdc_raw.is_null() {
            print_last_error_message!();
            crate::bail!("GetDC failed");
        }

        let hdc = OwnedHDC { hdc: hdc_raw, hwnd };

        if GetPixelFormat(*hdc) == 0 {
            set_pixel_format(*hdc)?;
        }

        let dummy_raw = wglCreateContext(*hdc);
        if dummy_raw.is_null() {
            print_last_error_message!();
            crate::bail!("wglCreateContext create dummy failed");
        }

        let dummy = OwnedHGLRC(dummy_raw);

        if wglMakeCurrent(*hdc, *dummy) == 0 {
            print_last_error_message!();
            crate::bail!("wglMakeCurrent failed");
        }

        type WglCreateContextAttribsARB =
            unsafe extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC;

        let Some(proc) = wglGetProcAddress(s!("wglCreateContextAttribsARB")) else {
            crate::bail!("Get 'wglCreateContextAttribsARB' failed");
        };

        let wgl_create_context_attribs_arb: WglCreateContextAttribsARB = core::mem::transmute(proc);

        // OpenGL 3.3 core
        const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
        const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;
        const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;
        const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x00000001;

        let attribs = [
            WGL_CONTEXT_MAJOR_VERSION_ARB,
            3,
            WGL_CONTEXT_MINOR_VERSION_ARB,
            3,
            WGL_CONTEXT_PROFILE_MASK_ARB,
            WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
            0,
        ];

        let modern_raw =
            wgl_create_context_attribs_arb(*hdc, core::ptr::null_mut(), attribs.as_ptr());
        if modern_raw.is_null() {
            print_last_error_message!();
            crate::bail!("Create modern context failed");
        }

        let modern = OwnedHGLRC(modern_raw);

        if wglMakeCurrent(*hdc, modern.0) == 0 {
            print_last_error_message!();
            crate::bail!("wglMakeCurrent failed");
        }

        Ok((hdc, modern))
    }
}

/// 基于已激活的WGL上下文创建`glow::Context`
unsafe fn create_glow_context() -> crate::Result<glow::Context> {
    unsafe {
        let gl_mod = crate::utils::win32::get_module_handle(w!("opengl32.dll"))?;

        let ctx = glow::Context::from_loader_function(|s| {
            let buf = s.as_bytes().with_null();
            if let Some(addr) = wglGetProcAddress(buf.as_ptr()) {
                return addr as _;
            }

            match crate::utils::win32::get_module_symbol_addr_from_handle(gl_mod, buf.as_ptr()) {
                Ok(addr) => addr as _,
                Err(_) => core::ptr::null(),
            }
        });

        Ok(ctx)
    }
}

/// 编译并创建着色器对象。
///
/// # 参数
/// - `gl`: OpenGL 上下文引用，用于执行 GPU 操作
/// - `shader_type`: 着色器类型，应为 `glow::VERTEX_SHADER` 或 `glow::FRAGMENT_SHADER` 等常量
/// - `source`: GLSL 源代码字符串
pub fn compile_shader(
    gl: &Arc<glow::Context>,
    shader_type: u32,
    source: &str,
) -> crate::Result<OwnedShader> {
    unsafe {
        let raw_shader = gl
            .create_shader(shader_type)
            .map_err(|e| crate::anyhow!("Shader creation failed: {}", e))?;
        let shader = OwnedShader {
            gl: gl.clone(),
            shader: raw_shader,
        };
        gl.shader_source(*shader, source);
        gl.compile_shader(*shader);

        if !gl.get_shader_compile_status(*shader) {
            crate::bail!("Shader compile error: {}", gl.get_shader_info_log(*shader));
        }

        Ok(shader)
    }
}

/// 创建顶点数组对象（VAO）。
///
/// VAO 用于存储顶点属性配置（如顶点坐标、颜色、法线的内存布局），
/// 绑定 VAO 后可快速切换整组顶点状态，避免重复配置。
pub fn create_vertex_array(gl: &Arc<glow::Context>) -> crate::Result<OwnedVertexArray> {
    let raw_vao = unsafe {
        gl.create_vertex_array()
            .map_err(|e| crate::anyhow!("Vertex array creation failed: {e}"))?
    };

    Ok(OwnedVertexArray {
        gl: gl.clone(),
        vao: raw_vao,
    })
}

/// 创建 GPU 缓冲区对象（VBO/EBO 通用）。
///
/// 缓冲区用于存储顶点数据（坐标、颜色、UV）或索引数据，
/// 创建后需通过 `gl.bind_buffer()` 和 `gl.buffer_data()` 上传实际数据。
pub fn create_buffer(gl: &Arc<glow::Context>) -> crate::Result<OwnedBuffer> {
    let raw_buffer = unsafe {
        gl.create_buffer()
            .map_err(|e| crate::anyhow!("Buffer creation failed: {e}"))?
    };
    Ok(OwnedBuffer {
        gl: gl.clone(),
        buffer: raw_buffer,
    })
}

/// 创建着色器程序对象。
///
/// 程序对象用于链接多个着色器阶段（顶点、片段、几何等），
/// 是 GPU 渲染管线的最终执行单元。
pub fn create_program(gl: &Arc<glow::Context>) -> crate::Result<OwnedProgram> {
    let raw_program = unsafe {
        gl.create_program()
            .map_err(|e| crate::anyhow!("Program creation failed: {}", e))?
    };

    Ok(OwnedProgram {
        gl: gl.clone(),
        program: raw_program,
    })
}

/// 创建 GPU 纹理对象。
///
/// 纹理用于存储图像数据（颜色、深度、模板等），可被着色器采样。
/// 创建后需通过 `gl.bind_texture()` 和 `gl.tex_image_2d()` 等函数
/// 上传实际数据并配置纹理参数。
pub fn create_texture(gl: &Arc<glow::Context>) -> crate::Result<OwnedTexture> {
    let raw_texture = unsafe {
        gl.create_texture()
            .map_err(|e| crate::anyhow!("Texture creation failed: {}", e))?
    };

    Ok(OwnedTexture {
        gl: gl.clone(),
        texture: raw_texture,
    })
}
