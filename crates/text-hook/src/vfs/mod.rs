mod pattern;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use windows_sys::Win32::Storage::FileSystem::WIN32_FIND_DATAW;

use crate::{
    utils::exts::{ptr_ext::PtrExt, slice_ext::WideSliceExt},
    vfs::pattern::{PatternMatcher, PatternTemplate},
};

mod rules {
    use super::{RawVfsRule, VfsMode};

    translate_macros::generate_vfs_rules!(
        "assets/vfs_rules.json",
        "constant_assets/vfs_rules.json"
    );
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum VfsMode {
    /// 目标文件不存在时回退到源路径
    Fallback,
    /// 强制映射, 不管目标文件是否存在
    Force,
}

#[derive(Clone, Copy, Debug)]
struct RawVfsRule {
    pub source: &'static str,
    pub target: &'static str,
    pub mode: VfsMode,
}

/// 编译并展开变量后的规则
struct ResolvedRule {
    matcher: PatternMatcher,
    template: PatternTemplate,
    mode: VfsMode,
}

/// 获取默认变量表
fn default_vars() -> Vec<(&'static str, String)> {
    let mut vars = Vec::with_capacity(4);

    let cwd = crate::utils::win32::get_current_dir(false)
        .map_or_else(|_| ".".to_string(), |v| v.to_string_lossy());

    let temp = crate::utils::win32::get_temp_dir(false)
        .map_or_else(|_| ".".to_string(), |v| v.to_string_lossy());

    let exe_dir = crate::utils::get_executable_dir()
        .to_string_lossy()
        .into_owned();

    vars.push(("{cwd}", cwd));
    vars.push(("{temp_dir}", temp));
    vars.push(("{exe_dir}", exe_dir));

    #[cfg(feature = "enable_resource_pack")]
    {
        vars.push((
            "{resource_pack_dir}",
            crate::resource_pack::get_temp_dir()
                .to_string_lossy()
                .into_owned(),
        ));
    }

    vars
}

/// 将变量占位符替换为实际路径
fn expand_vars(template: &str, vars: &[(&'static str, String)]) -> String {
    let mut result = template.to_string();
    for (var, path) in vars {
        // 只有当确定存在时才进行分配替换，避免无意义的遍历开销
        if result.contains(var) {
            result = result.replace(var, path);
        }
    }
    result
}

/// 将 `RawVfsRule` 编译为 `ResolvedRule`
fn resolve_rule(raw: &RawVfsRule, vars: &[(&'static str, String)]) -> ResolvedRule {
    let clean_source = to_unix_clean_path_string(&expand_vars(raw.source, vars));
    let clean_target = to_unix_clean_path_string(&expand_vars(raw.target, vars));

    ResolvedRule {
        matcher: PatternMatcher::compile(&clean_source),
        template: PatternTemplate::compile(&clean_target),
        mode: raw.mode,
    }
}

/// 全局编译后的规则表
static RULES: LazyLock<Vec<ResolvedRule>> = LazyLock::new(|| {
    let vars = default_vars();

    for dir in rules::CREATE_DIRS {
        let expanded = expand_vars(dir, &vars);
        if let Err(e) = std::fs::create_dir_all(&expanded) {
            crate::debug!("VFS create_dir_all failed for `{expanded}`: {e}");
        }
    }

    rules::VFS_RULES
        .iter()
        .map(|raw| resolve_rule(raw, &vars))
        .collect()
});

struct RedirectResult {
    target_path: PathBuf,
    mode: VfsMode,
}

/// 在规则表中查找匹配的路径规则，返回目标路径和模式。
fn resolve_redirect(path: &Path) -> Option<RedirectResult> {
    let clean_path = normalize_path_to_string(path);

    crate::debug!("VFS resolve: {clean_path}");

    for rule in RULES.iter() {
        if let Some(captures) = rule.matcher.match_path(&clean_path) {
            let target_str = rule.template.fill(&captures);

            crate::debug!(
                "VFS rule matched: {} -> {} (mode: {:?})",
                path.display(),
                target_str,
                rule.mode
            );

            let target_path = to_windows_path(&target_str);
            return Some(RedirectResult {
                target_path,
                mode: rule.mode,
            });
        }
    }

    None
}

/// 尝试将 path 重定向到 VFS 规则中定义的目标路径
///
/// 返回 `Some(target_path)` 表示命中规则并成功映射, `None` 表示未命中任何规则
/// 或 fallback 模式下目标文件不存在。
///
/// `target_path` 已经转为 `\\?\` 格式（如为绝对路径），可直接传给 Win32 API。
pub fn try_redirect(path: &Path) -> crate::Result<Option<PathBuf>> {
    let Some(RedirectResult { target_path, mode }) = resolve_redirect(path) else {
        return Ok(None);
    };

    crate::debug!("calling `try_redirect` with mode: {mode:?}");

    match mode {
        VfsMode::Force => Ok(Some(target_path)),
        VfsMode::Fallback => {
            if target_path.try_exists()? {
                Ok(Some(target_path))
            } else {
                crate::debug!("VFS fallback: target not found, using original path");
                Ok(None)
            }
        }
    }
}

/// 从 `WIN32_FIND_DATAW` 中提取文件名（用于去重比较）。
#[allow(dead_code)]
fn extract_filename(data: &WIN32_FIND_DATAW) -> String {
    let cfile = unsafe {
        data.cFileName
            .as_ptr()
            .to_slice_until_null(data.cFileName.len() - 1)
    };
    cfile.to_string_lossy().to_ascii_lowercase()
}

/// 对 `FindFirstFile` 系列 API 执行 VFS 路径重定向与文件枚举。
///
/// `get_snapshot` 接受一个路径，调用原生 API（如 `FindFirstFileW` 或
/// `FindFirstFileExW`）并返回该路径匹配的所有 `WIN32_FIND_DATAW` 条目。
///
/// 返回 `None` 表示无规则命中，调用方应直接 fallback 到原函数。
///
/// - Force 模式下仅使用重定向后的路径调用 `get_snapshot`。
/// - Fallback 模式下合并两端结果，重复文件名以重定向优先。
#[allow(dead_code)]
pub fn try_enum<F>(path: &Path, get_snapshot: F) -> crate::Result<Option<Vec<WIN32_FIND_DATAW>>>
where
    F: Fn(&Path) -> crate::Result<Vec<WIN32_FIND_DATAW>>,
{
    let Some(RedirectResult { target_path, mode }) = resolve_redirect(path) else {
        return Ok(None);
    };

    crate::debug!("calling `try_enum` with mode: {mode:?}");

    match mode {
        VfsMode::Force => get_snapshot(&target_path).map(Some),
        VfsMode::Fallback => {
            let original = get_snapshot(path).unwrap_or_default();
            let redirected = get_snapshot(&target_path).unwrap_or_default();

            let mut seen = HashSet::new();
            let mut result = Vec::with_capacity(original.len() + redirected.len());

            for entry in redirected {
                seen.insert(extract_filename(&entry));
                result.push(entry);
            }

            for entry in original {
                if seen.insert(extract_filename(&entry)) {
                    result.push(entry);
                }
            }

            Ok(Some(result))
        }
    }
}

/// 将路径转为绝对路径字符串并进行规范化
fn normalize_path_to_string(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    to_unix_clean_path_string(&path_clean::clean(absolute).to_string_lossy())
}

/// 将路径字符串规范化为统一的小写 Unix 风格路径。
///
/// 优先剥离真实的 Windows 前缀，随后替换斜杠，确保逻辑的严密性。
fn to_unix_clean_path_string(s: &str) -> String {
    let mut path_str = s;

    // 正确的剥离顺序：先处理 Windows 独有前缀，再处理斜杠
    if let Some(stripped) = path_str.strip_prefix(r"\\?\UNC\") {
        path_str = stripped;
    } else if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
        path_str = stripped;
    } else if let Some(stripped) = path_str.strip_prefix(r"\\.\") {
        path_str = stripped;
    }

    let clean = path_str.replace('\\', "/").to_ascii_lowercase();
    clean.trim_end_matches('/').to_string()
}

/// 将路径字符串转为 Windows 风格的 `PathBuf`
///
/// 仅在确认路径为绝对路径时才应用 `\\?\` 长路径前缀，避免破坏相对路径。
fn to_windows_path(s: &str) -> PathBuf {
    let s_win = s.replace('/', "\\");
    let path = Path::new(&s_win);

    if path.is_absolute() {
        let final_s = if s_win.starts_with(r"\\?\") {
            s_win
        } else if let Some(rest) = s_win.strip_prefix(r"\\") {
            format!(r"\\?\UNC\{rest}")
        } else {
            format!(r"\\?\{s_win}")
        };
        PathBuf::from(final_s)
    } else {
        PathBuf::from(s_win)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_unix_clean_path_string() {
        // 1. 普通绝对路径与大小写、斜杠转换
        assert_eq!(to_unix_clean_path_string(r"C:\Games\Data"), "c:/games/data");
        assert_eq!(
            to_unix_clean_path_string(r"d:/games/data/"),
            "d:/games/data"
        );

        // 2. 长路径前缀剥离 (\\?\)
        assert_eq!(
            to_unix_clean_path_string(r"\\?\C:\Users\Admin"),
            "c:/users/admin"
        );
        assert_eq!(
            to_unix_clean_path_string(r"\\?\E:\Game/Mods/"),
            "e:/game/mods"
        );

        // 3. UNC 长路径前缀剥离 (\\?\UNC\)
        assert_eq!(
            to_unix_clean_path_string(r"\\?\UNC\server\share\file"),
            "server/share/file"
        );

        // 4. NT 设备路径前缀剥离 (\\.\)
        assert_eq!(
            to_unix_clean_path_string(r"\\.\PhysicalDrive0"),
            "physicaldrive0"
        );

        // 5. 相对路径与末尾斜杠清理
        assert_eq!(
            to_unix_clean_path_string(r"local\path\to\/"),
            "local/path/to"
        );
        assert_eq!(to_unix_clean_path_string("."), ".");
    }

    #[test]
    fn test_to_windows_path() {
        // 1. 普通绝对路径 -> 转换为带 \\?\ 前缀的 Windows 长路径
        assert_eq!(
            to_windows_path("C:/Games/Data"),
            PathBuf::from(r"\\?\C:\Games\Data")
        );
        assert_eq!(
            to_windows_path(r"D:\Games\Data"),
            PathBuf::from(r"\\?\D:\Games\Data")
        );

        // 2. 已经是 \\?\ 前缀的绝对路径 -> 保持原样，仅规范化斜杠
        assert_eq!(
            to_windows_path(r"\\?\C:\Users\Admin"),
            PathBuf::from(r"\\?\C:\Users\Admin")
        );
        assert_eq!(
            to_windows_path(r"\\?/C:/Users/Admin"),
            PathBuf::from(r"\\?\C:\Users\Admin")
        );

        // 3. 标准网络 UNC 路径 (\\server\share) -> 转换为 \\?\UNC\server\share
        assert_eq!(
            to_windows_path(r"\\server\share\file"),
            PathBuf::from(r"\\?\UNC\server\share\file")
        );
        assert_eq!(
            to_windows_path("//server/share/file"),
            PathBuf::from(r"\\?\UNC\server\share\file")
        );

        // 4. 相对路径 -> 绝不添加 \\?\ 前缀，仅规范化斜杠
        assert_eq!(
            to_windows_path("relative/path/to/file"),
            PathBuf::from(r"relative\path\to\file")
        );
        assert_eq!(
            to_windows_path(r"some\local\dir"),
            PathBuf::from(r"some\local\dir")
        );
        assert_eq!(to_windows_path("."), PathBuf::from("."));
    }
}
