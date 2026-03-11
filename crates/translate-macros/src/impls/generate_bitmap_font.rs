use proc_macro2::{Literal, TokenStream};
use quote::quote;
use serde::Deserialize;
use std::collections::HashSet;
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};

use crate::impls::utils::get_full_path_by_manifest;

struct PathInput {
    path: LitStr,
}

impl Parse for PathInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        Ok(PathInput { path })
    }
}

#[derive(Deserialize)]
pub struct FontConfig {
    pub font_path: String,
    pub chars: String,
    #[serde(default = "defaults::font_size")]
    pub font_size: u32,
    #[serde(default = "defaults::padding")]
    pub padding: u32,
    #[serde(default = "defaults::texture_max_width")]
    pub texture_max_width: u32,
}

mod defaults {
    pub fn font_size() -> u32 {
        24
    }
    pub fn padding() -> u32 {
        2
    }
    pub fn texture_max_width() -> u32 {
        2048
    }
}

pub fn generate_bitmap_font(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathInput>(input)?;

    let path = get_full_path_by_manifest(parsed.path.value())?;
    let json_str = std::fs::read_to_string(&path)
        .map_err(|e| syn_err2!("无法读取配置 {}: {}", path.display(), e))?;

    let font_config: FontConfig = serde_json::from_str(&json_str)
        .map_err(|e| syn_err2!("解析默认配置 JSON 失败 ({}): {}", path.display(), e))?;

    let font_data = std::fs::read(get_full_path_by_manifest(&font_config.font_path)?)
        .map_err(|e| syn_err2!("读取字体文件 '{}' 得到错误 {e}", font_config.font_path))?;

    let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
        .map_err(|e| syn_err2!("字体解析失败: {e}"))?;

    // --------------------------------
    // chars 去重（保持顺序）
    // --------------------------------

    let mut seen = HashSet::new();
    let mut chars = Vec::new();

    for c in font_config.chars.chars() {
        if seen.insert(c) {
            chars.push(c);
        }
    }

    // --------------------------------
    // rasterize
    // --------------------------------

    struct Glyph {
        ch: char,
        metrics: fontdue::Metrics,
        bitmap: Vec<u8>,
    }

    let mut glyphs = Vec::new();

    for ch in chars {
        let (metrics, bitmap) = font.rasterize(ch, font_config.font_size as f32);
        glyphs.push(Glyph {
            ch,
            metrics,
            bitmap,
        });
    }

    // --------------------------------
    // atlas packing
    // --------------------------------

    let padding = font_config.padding as usize;
    let max_width = font_config.texture_max_width as usize;

    let mut cursor_x = 1; // (0,0) 留给 WHITE_PIXEL
    let mut cursor_y = 0;
    let mut row_height = 0;

    struct PlacedGlyph {
        ch: char,
        metrics: fontdue::Metrics,
        bitmap: Vec<u8>,
        x: usize,
        y: usize,
    }

    let mut placed = Vec::new();

    for g in glyphs {
        let w = g.metrics.width + padding * 2;
        let h = g.metrics.height + padding * 2;

        if w > max_width {
            syn_bail2!("glyph '{}' 超出 atlas 最大宽度", g.ch);
        }

        if cursor_x + w > max_width {
            cursor_x = 1;
            cursor_y += row_height;
            row_height = 0;
        }

        placed.push(PlacedGlyph {
            ch: g.ch,
            metrics: g.metrics,
            bitmap: g.bitmap,
            x: cursor_x,
            y: cursor_y,
        });

        cursor_x += w;
        row_height = row_height.max(h);
    }

    let atlas_width = max_width;
    let atlas_height = cursor_y + row_height;

    if atlas_height > 8192 {
        return Err(syn_err2!("atlas 高度 {} 超出限制", atlas_height));
    }

    // --------------------------------
    // atlas buffer
    // --------------------------------

    let mut atlas = vec![0u8; atlas_width * atlas_height];

    // WHITE_PIXEL
    atlas[0] = 255;

    for g in &placed {
        let metrics = &g.metrics;
        let bw = metrics.width;
        let bh = metrics.height;

        for row in 0..bh {
            for col in 0..bw {
                let src = row * bw + col;

                let dst_x = g.x + padding + col;
                let dst_y = g.y + padding + row;

                let dst = dst_y * atlas_width + dst_x;

                atlas[dst] = g.bitmap[src];
            }
        }
    }

    // --------------------------------
    // 写 atlas 文件
    // --------------------------------

    let atlas_path = get_full_path_by_manifest("assets/temp/bitmap_font.bin")?;
    std::fs::create_dir_all(atlas_path.parent().unwrap()).ok();

    std::fs::write(&atlas_path, &atlas).map_err(|e| syn_err2!("写 atlas 失败: {e}"))?;

    // --------------------------------
    // 生成 phf
    // --------------------------------

    let mut entries = Vec::new();

    for g in &placed {
        let m = &g.metrics;

        let x = (g.x + padding) as f32;
        let y = (g.y + padding) as f32;
        let w = m.width as f32;
        let h = m.height as f32;

        let uv_min_x = x / atlas_width as f32;
        let uv_min_y = y / atlas_height as f32;
        let uv_max_x = (x + w) / atlas_width as f32;
        let uv_max_y = (y + h) / atlas_height as f32;
        let width = m.width as u32;
        let height = m.height as u32;
        let xmin = m.xmin;
        let ymin = m.ymin;
        let advance = m.advance_width;

        let ch = g.ch;

        entries.push(quote! {
            #ch => CharInfo {
                uv_min: [#uv_min_x, #uv_min_y],
                uv_max: [#uv_max_x, #uv_max_y],
                width: #width,
                height: #height,
                xmin: #xmin,
                ymin: #ymin,
                advance: #advance,
            }
        });
    }

    // WHITE_PIXEL token

    let white_uv_max_x = 1.0 / atlas_width as f32;
    let white_uv_max_y = 1.0 / atlas_height as f32;

    let white_pixel = quote! {
        pub const WHITE_PIXEL: CharInfo = CharInfo {
            uv_min: [0.0, 0.0],
            uv_max: [#white_uv_max_x, #white_uv_max_y],
            width: 1,
            height: 1,
            xmin: 0,
            ymin: 0,
            advance: 1.0,
        };
    };

    let atlas_width_lit = Literal::u32_unsuffixed(atlas_width as u32);
    let atlas_height_lit = Literal::u32_unsuffixed(atlas_height as u32);

    // --------------------------------
    // vertical metrics
    // --------------------------------

    let vm = font
        .horizontal_line_metrics(font_config.font_size as f32)
        .ok_or_else(|| syn_err2!("字体缺少垂直度量信息"))?;

    let ascent = vm.ascent.ceil() as i32;
    let descent = vm.descent.floor() as i32;
    let line_height = (ascent - descent) as usize;

    // --------------------------------
    // TokenStream
    // --------------------------------

    Ok(quote! {
        ::translate_macros::embed!(
            pub(super) static BITMAP_FONT: [u8]
            from "assets/temp/bitmap_font.bin"
        );

        pub(super) const ATLAS_WIDTH: u32 = #atlas_width_lit;
        pub(super) const ATLAS_HEIGHT: u32 = #atlas_height_lit;

        pub(super) const ASCENT: i32 = #ascent;
        pub(super) const DESCENT: i32 = #descent;
        pub(super) const LINE_HEIGHT: usize = #line_height;

        #white_pixel

        pub(super) static CHAR_MAP: ::phf::Map<char, CharInfo> = ::phf::phf_map! {
            #(#entries),*
        };
    })
}
