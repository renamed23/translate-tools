use bytemuck::{Pod, Zeroable};
use glow::HasContext;
use std::sync::Arc;

use crate::{
    gl::{compile_shader, create_buffer, create_program, create_texture, create_vertex_array},
    utils::raii_wrapper::{OwnedBuffer, OwnedProgram, OwnedTexture, OwnedVertexArray},
};

mod bitmap_font {
    use super::CharInfo;
    translate_macros::generate_bitmap_font!("assets/bitmap_font.json");
}

/// 顶点数据结构，用于 GPU 渲染
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// 屏幕坐标 (像素单位，左上角为原点)
    pub pos: [f32; 2],

    /// 纹理坐标 (0.0 - 1.0 范围)
    pub uv: [f32; 2],

    /// 颜色数据，RGBA8 格式
    /// 使用 u8 而非 f32 可节省 75% 显存带宽
    pub color: [u8; 4],
}

/// 单个字符的字体元数据
pub struct CharInfo {
    /// 纹理坐标左上角
    pub uv_min: [f32; 2],

    /// 纹理坐标右下角
    pub uv_max: [f32; 2],

    /// 字符位图宽度 (像素)
    pub width: u32,

    /// 字符位图高度 (像素)
    pub height: u32,

    /// 水平偏移量 (相对于基线)
    pub xmin: i32,

    /// 垂直偏移量 (相对于基线，向上为正)
    pub ymin: i32,

    /// 水平步进值 (下一个字符的起始位置偏移)
    pub advance: f32,
}

/// OpenGL 2D 渲染器
///
/// 负责管理着色器、顶点数据、字体纹理，提供基础的矩形和文本绘制功能
pub struct GLPainter {
    program: OwnedProgram,
    vao: OwnedVertexArray,
    vbo: OwnedBuffer,
    ebo: OwnedBuffer,
    font_texture: OwnedTexture,
    gl: Arc<glow::Context>,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    u_screen_size_loc: glow::UniformLocation,
}

const VS_SRC: &str = r#"#version 330 core
        layout (location = 0) in vec2 a_pos;
        layout (location = 1) in vec2 a_uv;
        layout (location = 2) in vec4 a_color;
        
        out vec2 v_uv;
        out vec4 v_color;
        uniform vec2 u_screen_size;

        void main() {
            vec2 ndc = (a_pos / u_screen_size) * 2.0 - 1.0;
            gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
            v_uv = a_uv;
            v_color = a_color;
        }"#;

const FS_SRC: &str = r#"#version 330 core
        in vec2 v_uv;
        in vec4 v_color;
        uniform sampler2D u_sampler;
        out vec4 f_color;

        void main() {
            float alpha = texture(u_sampler, v_uv).r;
            f_color = vec4(v_color.rgb, v_color.a * alpha);
        }"#;

impl GLPainter {
    /// 创建新的 GLPainter 实例
    ///
    /// 初始化流程：
    /// 1. 编译链接着色器
    /// 2. 创建并配置 VAO/VBO/EBO
    /// 3. 上传字体纹理数据
    pub fn new(gl: Arc<glow::Context>) -> crate::Result<Self> {
        let vs = compile_shader(&gl, glow::VERTEX_SHADER, VS_SRC)?;
        let fs = compile_shader(&gl, glow::FRAGMENT_SHADER, FS_SRC)?;

        let program = create_program(&gl)?;

        unsafe {
            gl.attach_shader(*program, *vs);
            gl.attach_shader(*program, *fs);
            gl.link_program(*program);

            if !gl.get_program_link_status(*program) {
                crate::bail!("Program link error: {}", gl.get_program_info_log(*program));
            }

            gl.detach_shader(*program, *vs);
            gl.detach_shader(*program, *fs);
        }

        let u_screen_size_loc = unsafe {
            gl.get_uniform_location(*program, "u_screen_size")
                .ok_or_else(|| crate::anyhow!("Uniform not found"))?
        };

        let vao = create_vertex_array(&gl)?;
        let vbo = create_buffer(&gl)?;
        let ebo = create_buffer(&gl)?;

        unsafe {
            gl.bind_vertex_array(Some(*vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(*vbo));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*ebo));

            let stride = core::mem::size_of::<Vertex>() as i32;

            // 0: pos (vec2, f32)
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);

            // 1: uv (vec2, f32)
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);

            // 2: color (rgba, u8)
            // 关键：使用 UNSIGNED_BYTE 类型，normalized=true 将 0-255 映射到 0.0-1.0
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::UNSIGNED_BYTE, true, stride, 16);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }

        let font_texture = create_texture(&gl)?;

        unsafe {
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

            gl.bind_texture(glow::TEXTURE_2D, Some(*font_texture));
            // 上传单通道 R8 纹理数据
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,               // level 0 (base)
                glow::R8 as i32, // 内部格式：单通道 8bit
                bitmap_font::ATLAS_WIDTH as i32,
                bitmap_font::ATLAS_HEIGHT as i32,
                0,
                glow::RED,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&bitmap_font::BITMAP_FONT)),
            );

            // 最近邻过滤：保持像素清晰，适合位图字体
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );

            // 边缘钳制：避免纹理采样时产生边缘伪影
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
        };

        Ok(Self {
            program,
            vao,
            vbo,
            ebo,
            font_texture,
            gl,
            vertices: Vec::new(),
            indices: Vec::new(),
            u_screen_size_loc,
        })
    }

    /// 清空 CPU 端缓冲区
    ///
    /// 通常在每帧开始或 paint() 后调用
    pub fn clear_buf(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// 执行 GPU 绘制
    ///
    /// 流程：
    /// 1. 上传顶点/索引数据到 GPU
    /// 2. 绑定资源并设置 uniform
    /// 3. 调用 draw_elements
    /// 4. 清理状态并清空缓冲区
    pub fn paint(&mut self, screen_size: [f32; 2]) {
        if self.indices.is_empty() {
            return;
        }

        unsafe {
            // 1. 上传顶点数据
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(*self.vbo));
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&self.vertices),
                glow::DYNAMIC_DRAW,
            );

            self.gl
                .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*self.ebo));
            self.gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&self.indices),
                glow::DYNAMIC_DRAW,
            );

            // 2. 配置渲染状态
            self.gl.use_program(Some(*self.program));
            self.gl.bind_vertex_array(Some(*self.vao));

            self.gl.uniform_2_f32(
                Some(&self.u_screen_size_loc),
                screen_size[0],
                screen_size[1],
            );

            self.gl.active_texture(glow::TEXTURE0);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(*self.font_texture));

            // 3. 执行绘制调用
            self.gl.draw_elements(
                glow::TRIANGLES,
                self.indices.len() as i32,
                glow::UNSIGNED_INT,
                0,
            );

            // 4. 清理现场，避免影响后续渲染
            self.gl.bind_vertex_array(None);
            self.clear_buf();
        }
    }

    /// 添加一个四边形到缓冲区
    ///
    /// 生成 4 个顶点和 6 个索引 (两个三角形组成四边形)
    fn push_quad(
        &mut self,
        pos_min: [f32; 2], // 左上角屏幕坐标
        pos_max: [f32; 2], // 右下角屏幕坐标
        uv_min: [f32; 2],  // 左上角纹理坐标
        uv_max: [f32; 2],  // 右下角纹理坐标
        color: [u8; 4],    // RGBA 颜色
    ) {
        let base = self.vertices.len() as u32;

        self.vertices.extend_from_slice(&[
            Vertex {
                pos: [pos_min[0], pos_min[1]],
                uv: [uv_min[0], uv_min[1]],
                color,
            },
            Vertex {
                pos: [pos_max[0], pos_min[1]],
                uv: [uv_max[0], uv_min[1]],
                color,
            },
            Vertex {
                pos: [pos_max[0], pos_max[1]],
                uv: [uv_max[0], uv_max[1]],
                color,
            },
            Vertex {
                pos: [pos_min[0], pos_max[1]],
                uv: [uv_min[0], uv_max[1]],
                color,
            },
        ]);

        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// 绘制纯色矩形
    ///
    /// 使用字体纹理中的白色像素 (WHITE_PIXEL) 作为遮罩，
    /// 通过顶点颜色控制最终显示颜色
    ///
    /// # 参数
    /// - `pos`: 起始位置（屏幕坐标系）
    /// - `size`: 矩形大小
    /// - `color`: 矩形颜色
    pub fn add_rect(&mut self, pos: [f32; 2], size: [f32; 2], color: [u8; 4]) {
        let pos_min = pos;
        let pos_max = [pos[0] + size[0], pos[1] + size[1]];

        // 使用纹理中预定义的白色像素区域
        let uv_min = bitmap_font::WHITE_PIXEL.uv_min;
        let uv_max = bitmap_font::WHITE_PIXEL.uv_max;

        self.push_quad(pos_min, pos_max, uv_min, uv_max, color);
    }

    /// 绘制单个字符
    ///
    /// # 参数
    /// - `pos`: 起始位置 (屏幕坐标系)
    /// - `ch`: 要绘制的字符
    /// - `color`: 文字颜色
    ///
    /// # 返回值
    /// - 水平步进值 (advance)，用于计算下一个字符位置
    pub fn add_char_rect(&mut self, pos: [i32; 2], ch: char, color: [u8; 4]) -> f32 {
        let info = match bitmap_font::CHAR_MAP.get(&ch) {
            Some(i) => i,
            None => return 0.0,
        };

        // 计算基线位置（相对于传入的 pos 偏移 ASCENT）
        let baseline = [pos[0], pos[1] + bitmap_font::ASCENT];

        // 位图坐标是左上到右下，与uv坐标保持一致
        let x = baseline[0] + info.xmin;
        let y = baseline[1] - info.ymin - info.height as i32;

        let w = info.width as i32;
        let h = info.height as i32;

        let pos_min = [x as f32, y as f32];
        let pos_max = [(x + w) as f32, (y + h) as f32];

        self.push_quad(pos_min, pos_max, info.uv_min, info.uv_max, color);

        info.advance
    }

    /// 绘制文本字符串
    ///
    /// 支持换行符 '\n' 进行多行文本渲染
    /// 使用 add_char_rect 逐个字符绘制，自动处理字距
    ///
    /// # 参数
    /// - `pos`: 起始位置（屏幕坐标系）
    /// - `text`: 文本
    /// - `color`: 文本颜色
    pub fn add_text(&mut self, pos: [f32; 2], text: &str, color: [u8; 4]) {
        let mut cur_pos_x = pos[0];
        let mut cur_pos_y = pos[1];

        for ch in text.chars() {
            if ch == '\n' {
                cur_pos_x = pos[0];
                cur_pos_y += bitmap_font::LINE_HEIGHT as f32;
                continue;
            }

            let advance = self.add_char_rect(
                [cur_pos_x.round() as i32, cur_pos_y.round() as i32],
                ch,
                color,
            );
            cur_pos_x += advance;
        }
    }
}
