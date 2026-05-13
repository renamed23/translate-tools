use std::hash::Hash;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Graphics::Gdi::LOGFONTW;

use crate::{
    constant::{
        CHAR_SET, CREATE_FONT_B_ITALIC, CREATE_FONT_B_STRIKE_OUT, CREATE_FONT_B_UNDERLINE,
        CREATE_FONT_C_ESCAPEMENT, CREATE_FONT_C_HEIGHT, CREATE_FONT_C_ORIENTATION,
        CREATE_FONT_C_WEIGHT, CREATE_FONT_C_WIDTH, CREATE_FONT_I_CLIP_PRECISION,
        CREATE_FONT_I_OUT_PRECISION, CREATE_FONT_I_PITCH_AND_FAMILY, CREATE_FONT_I_QUALITY,
        FONT_FACE,
    },
    utils::exts::slice_ext::CommonSliceExt,
};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct LogFont {
    pub height: i32,
    pub width: i32,
    pub escapement: i32,
    pub orientation: i32,
    pub weight: i32,
    pub italic: u8,
    pub underline: u8,
    pub strike_out: u8,
    pub char_set: u8,
    pub out_precision: u8,
    pub clip_precision: u8,
    pub quality: u8,
    pub pitch_and_family: u8,
    pub face_name: [u16; 32],
}

impl Default for LogFont {
    fn default() -> Self {
        let mut face_name = [0u16; 32];
        face_name.copy_min_from_slice_with_null(FONT_FACE);

        Self {
            height: CREATE_FONT_C_HEIGHT.unwrap_or(0),
            width: CREATE_FONT_C_WIDTH.unwrap_or(0),
            escapement: CREATE_FONT_C_ESCAPEMENT.unwrap_or(0),
            orientation: CREATE_FONT_C_ORIENTATION.unwrap_or(0),
            weight: CREATE_FONT_C_WEIGHT.unwrap_or(0),
            italic: CREATE_FONT_B_ITALIC.unwrap_or(0) as u8,
            underline: CREATE_FONT_B_UNDERLINE.unwrap_or(0) as u8,
            strike_out: CREATE_FONT_B_STRIKE_OUT.unwrap_or(0) as u8,
            char_set: CHAR_SET,
            out_precision: CREATE_FONT_I_OUT_PRECISION.unwrap_or(0) as u8,
            clip_precision: CREATE_FONT_I_CLIP_PRECISION.unwrap_or(0) as u8,
            quality: CREATE_FONT_I_QUALITY.unwrap_or(0) as u8,
            pitch_and_family: CREATE_FONT_I_PITCH_AND_FAMILY.unwrap_or(0) as u8,
            face_name,
        }
    }
}

impl LogFont {
    /// 转换为 windows-sys 的 LOGFONTW
    pub const fn to_sys_w(&self) -> LOGFONTW {
        LOGFONTW {
            lfHeight: self.height,
            lfWidth: self.width,
            lfEscapement: self.escapement,
            lfOrientation: self.orientation,
            lfWeight: self.weight,
            lfItalic: self.italic,
            lfUnderline: self.underline,
            lfStrikeOut: self.strike_out,
            lfCharSet: self.char_set,
            lfOutPrecision: self.out_precision,
            lfClipPrecision: self.clip_precision,
            lfQuality: self.quality,
            lfPitchAndFamily: self.pitch_and_family,
            lfFaceName: self.face_name,
        }
    }

    /// 从 W 参数生成
    pub const fn from_sys_w(lf: &LOGFONTW) -> Self {
        Self {
            height: lf.lfHeight,
            width: lf.lfWidth,
            escapement: lf.lfEscapement,
            orientation: lf.lfOrientation,
            weight: lf.lfWeight,
            italic: lf.lfItalic,
            underline: lf.lfUnderline,
            strike_out: lf.lfStrikeOut,
            char_set: lf.lfCharSet,
            out_precision: lf.lfOutPrecision,
            clip_precision: lf.lfClipPrecision,
            quality: lf.lfQuality,
            pitch_and_family: lf.lfPitchAndFamily,
            face_name: lf.lfFaceName,
        }
    }
}
