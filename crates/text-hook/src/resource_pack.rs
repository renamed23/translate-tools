use std::path::{Path, PathBuf};

mod pack {
    translate_macros::generate_resource_pack!(
        "assets/resource_pack",
        crate::constant::RESOURCE_PACK_NAME
    );
}

/// 解压资源包到临时目录
pub fn extract() -> crate::Result<()> {
    pack::extract()
}

/// 清理资源包解压产生的临时文件
pub fn clean_up() -> crate::Result<()> {
    pack::clean_up()
}

fn to_unix_clean_path(path: &Path) -> String {
    let s = path.to_string_lossy();

    let mut s = s.to_lowercase().replace('\\', "/");

    // 1. 剥离前缀（因为已经全小写且换了斜杠，所以匹配 //?/）
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string()
    } else if let Some(stripped) = s.strip_prefix("//./") {
        s = stripped.to_string()
    }

    // 2. 强制加尾部斜杠用于前缀匹配，防止 MyGame 和 MyGameLauncher 混淆
    if !s.ends_with('/') && !s.is_empty() {
        s.push('/');
    }

    s
}

fn to_windows_path(path: &Path) -> String {
    // 确保使用反斜杠
    let mut s = path.to_string_lossy().replace('/', "\\");

    // 加上 \\?\ 前缀给 CreateFileW 使用
    if !s.starts_with(r"\\") {
        s = format!(r"\\?\{}", s)
    }

    s
}

/// 将可执行目录下的路径映射到资源包临时目录路径
///
/// 输入路径会被转为绝对路径，与可执行目录进行大小写不敏感的前缀匹配，
/// 匹配成功则在资源包中查找对应资源，返回资源的绝对路径
pub fn get_resource_path(path: &Path) -> crate::Result<Option<PathBuf>> {
    let exec_dir = crate::utils::get_executable_dir();

    let cwd = std::env::current_dir()
        .map_err(|e| crate::anyhow!("Get current work directory failed with : {e}"))?;

    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    let abs_path = path_clean::clean(&abs_path);

    crate::debug!(
        "Trying to get resource path for {}, abs_path={}, exec_dir={}",
        path.display(),
        abs_path.display(),
        exec_dir.display()
    );

    let clean_abs = to_unix_clean_path(&abs_path);
    let clean_exec = to_unix_clean_path(exec_dir);

    if clean_abs.starts_with(&clean_exec) {
        let mut relative_str = &clean_abs[clean_exec.len()..];
        if relative_str.starts_with('/') {
            relative_str = &relative_str[1..];
        }

        if relative_str.ends_with('/') {
            relative_str = &relative_str[..relative_str.len() - 1];
        }

        crate::debug!("Relative path for resource pack: {}", relative_str);

        if pack::is_resource(relative_str) {
            let temp_dir = pack::get_temp_dir();
            let final_path = temp_dir.join(relative_str);
            return Ok(Some(PathBuf::from(to_windows_path(&final_path))));
        }
    }

    Ok(None)
}
