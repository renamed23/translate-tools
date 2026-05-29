use windows_sys::Win32::Graphics::Gdi::{
    CreateFontA, DeleteObject, GetObjectA, LOGFONTA, LF_FACESIZE,
};

pub fn run_tests() -> anyhow::Result<()> {
    let features = std::env::var("TEXT_HOOK_FEATURES").unwrap_or_default();
    let disable_forced = features.contains("disable_forced_font");

    let arial_expected = if disable_forced { "Arial" } else { "SimHei" };
    test_font_face("Arial", arial_expected, "Arial")?;

    let msgothic_expected = if disable_forced { "SimHei" } else { "MS Gothic" };
    test_font_face("MS Gothic", msgothic_expected, "MS Gothic")?;

    Ok(())
}

fn test_font_face(input_face: &str, expected_face: &str, label: &str) -> anyhow::Result<()> {
    let input_ansi = to_ansi_null(input_face);

    unsafe {
        let hfont = CreateFontA(
            12, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 0, 0,
            input_ansi.as_ptr(),
        );

        if hfont.is_null() {
            anyhow::bail!("CreateFontA({label}) 失败");
        }

        let mut lf: LOGFONTA = std::mem::zeroed();
        let written = GetObjectA(
            hfont as _,
            std::mem::size_of::<LOGFONTA>() as i32,
            (&raw mut lf) as _,
        );
        DeleteObject(hfont as _);

        if written == 0 {
            anyhow::bail!("GetObjectA({label}) 失败");
        }

        let actual = logfont_face_to_string(&lf);

        if actual != expected_face {
            anyhow::bail!(
                "字体 {label}: 输入 '{input_face}', 期望 '{expected_face}', 实际 '{actual}'"
            );
        }
    }

    Ok(())
}

fn to_ansi_null(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn logfont_face_to_string(lf: &LOGFONTA) -> String {
    let face: &[u8] = unsafe {
        std::slice::from_raw_parts(lf.lfFaceName.as_ptr() as *const u8, LF_FACESIZE as usize)
    };
    let end = face.iter().position(|&b| b == 0).unwrap_or(face.len());
    String::from_utf8_lossy(&face[..end]).into_owned()
}
