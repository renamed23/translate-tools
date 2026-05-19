mod pattern;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::{
    utils::exts::slice_ext::WideSliceExt,
    vfs::pattern::{PatternMatcher, PatternTemplate},
};

mod rules {
    use super::{RawVfsRule, VfsMode};

    macro_rules! get_vfs_mode {
        ("fallback") => {
            VfsMode::Fallback
        };
        ("force") => {
            VfsMode::Force
        };
        ($s:literal) => {
            compile_error!(concat!("非法的 VfsMode: ", stringify!($s)));
        };
    }

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
fn default_vars() -> HashMap<String, String> {
    let mut vars = HashMap::with_capacity(4);

    let cwd = crate::utils::win32::get_current_dir(false)
        .map_or_else(|_| ".".to_string(), |v| v.to_string_lossy());

    let temp = crate::utils::win32::get_temp_dir(false)
        .map_or_else(|_| ".".to_string(), |v| v.to_string_lossy());

    let exe_dir = crate::utils::get_executable_dir()
        .to_string_lossy()
        .into_owned();

    vars.insert("{cwd}".to_string(), to_unix_clean_path_string(&cwd));
    vars.insert("{temp_dir}".to_string(), to_unix_clean_path_string(&temp));
    vars.insert("{exe_dir}".to_string(), to_unix_clean_path_string(&exe_dir));

    #[cfg(feature = "enable_resource_pack")]
    {
        vars.insert(
            "{resource_pack_dir}".to_string(),
            to_unix_clean_path_string(&crate::resource_pack::get_temp_dir().to_string_lossy()),
        );
    }

    vars
}

/// 将变量占位符替换为实际路径
fn expand_vars(template: &str, vars: &HashMap<String, String>) -> String {
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
fn resolve_rule(raw: &RawVfsRule, vars: &HashMap<String, String>) -> ResolvedRule {
    let source = expand_vars(raw.source, vars);
    let target = expand_vars(raw.target, vars);

    ResolvedRule {
        matcher: PatternMatcher::compile(&source),
        template: PatternTemplate::compile(&target),
        mode: raw.mode,
    }
}

/// 全局编译后的规则表
static RULES: LazyLock<Vec<ResolvedRule>> = LazyLock::new(|| {
    let vars = default_vars();
    rules::VFS_RULES
        .iter()
        .map(|raw| resolve_rule(raw, &vars))
        .collect()
});

/// 尝试将 path 重定向到 VFS 规则中定义的目标路径
///
/// 返回 `Some(target_path)` 表示命中规则并成功映射, `None` 表示未命中任何规则
/// 或 fallback 模式下目标文件不存在。
///
/// `target_path` 已经转为 `\\?\` 格式（如为绝对路径），可直接传给 Win32 API。
pub fn try_redirect(path: &Path) -> crate::Result<Option<PathBuf>> {
    let clean_path = normalize_path_to_string(path);

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

            match rule.mode {
                VfsMode::Fallback => {
                    if target_path.try_exists().unwrap_or(false) {
                        return Ok(Some(target_path));
                    }
                    crate::debug!("VFS fallback: target not found, using original path");
                    return Ok(None);
                }
                VfsMode::Force => {
                    return Ok(Some(target_path));
                }
            }
        }
    }

    Ok(None)
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
