/// 表示模式中的一个段
#[derive(Debug, Clone)]
enum Segment {
    /// 精确匹配 (不含 `*`)
    Literal(String),
    /// 包含单个 `*` 的段 (如 `*.png`, `name.*`, `*`)
    /// `*` 匹配的部分会被提取为一个捕获组
    Wildcard { prefix: String, suffix: String },
    /// 特殊允许的 `*.*`，拆分为两个捕获组 (stem 和 ext)
    StarDotStar,
    /// `**` (匹配零个或多个路径段, 含 `/`)
    RecursiveWild,
}

/// 编译后的源路径模式
#[derive(Debug, Clone)]
pub struct PatternMatcher {
    segments: Vec<Segment>,
}

/// 编译后的目标路径模板
#[derive(Debug, Clone)]
pub struct PatternTemplate {
    segments: Vec<Segment>,
}

/// 用于在回溯时避免分配内存的内部生命周期结构
enum CaptureRange<'a> {
    /// 对应 `Wildcard` 捕获的内容（不含前后缀）
    Single(&'a str),
    /// 对应 `RecursiveWild` 捕获的多级路径段
    Recursive(&'a [&'a str]),
}

impl CaptureRange<'_> {
    fn get_total(&self) -> String {
        match self {
            CaptureRange::Single(s) => s.to_string(),
            CaptureRange::Recursive(segs) => segs.join("/"),
        }
    }
}

impl PatternMatcher {
    /// 编译 source 模式字符串
    pub fn compile(source: &str) -> Self {
        Self {
            segments: parse_pattern(source),
        }
    }

    /// 匹配标准化后的路径, 返回每个捕获段的内容
    pub fn match_path(&self, normalized_path: &str) -> Option<Vec<String>> {
        let path_segs: Vec<&str> = normalized_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        // 预分配容量，*.* 会产生 2 个 capture，所以稍微留点冗余
        let mut captures = Vec::with_capacity(self.segments.len() * 2);

        if match_segments(&self.segments, 0, &path_segs, 0, &mut captures) {
            Some(captures.iter().map(CaptureRange::get_total).collect())
        } else {
            None
        }
    }
}

impl PatternTemplate {
    /// 编译 target 模板字符串
    pub fn compile(target: &str) -> Self {
        Self {
            segments: parse_pattern(target),
        }
    }

    /// 用捕获段填充模板, 生成目标路径
    pub fn fill(&self, captures: &[String]) -> String {
        fill_template(&self.segments, captures)
    }
}

/// 解析模式字符串为 Segment 序列
/// 依赖于过程宏已经排除了非法输入 (`*.*.*`, `**.png` 等)
fn parse_pattern(pattern: &str) -> Vec<Segment> {
    pattern
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| {
            if part == "**" {
                Segment::RecursiveWild
            } else if part == "*.*" {
                Segment::StarDotStar
            } else if let Some((prefix, suffix)) = part.split_once('*') {
                // 这个分支完美覆盖 `*.png`, `name.*`, 以及纯 `*`
                Segment::Wildcard {
                    prefix: prefix.to_string(),
                    suffix: suffix.to_string(),
                }
            } else {
                Segment::Literal(part.to_string())
            }
        })
        .collect()
}

/// 递归匹配段序列 (Zero-Allocation 回溯)
fn match_segments<'a>(
    segments: &[Segment],
    si: usize,
    path_segs: &'a [&'a str],
    pi: usize,
    captures: &mut Vec<CaptureRange<'a>>,
) -> bool {
    if si == segments.len() && pi == path_segs.len() {
        return true;
    }

    if si >= segments.len() {
        return false;
    }

    match &segments[si] {
        Segment::Literal(lit) => {
            if pi < path_segs.len() && path_segs[pi].eq_ignore_ascii_case(lit) {
                match_segments(segments, si + 1, path_segs, pi + 1, captures)
            } else {
                false
            }
        }
        Segment::Wildcard { prefix, suffix } => {
            if pi < path_segs.len() {
                let current_seg = path_segs[pi];
                let current_lower = current_seg.to_ascii_lowercase();
                let pre = prefix.to_ascii_lowercase();
                let suf = suffix.to_ascii_lowercase();

                if current_lower.starts_with(&pre)
                    && current_lower.ends_with(&suf)
                    && current_lower.len() >= pre.len() + suf.len()
                {
                    // 精准切割出中间被 `*` 匹配的部分
                    let capture_len = current_seg.len() - prefix.len() - suffix.len();
                    let capture_str = &current_seg[prefix.len()..prefix.len() + capture_len];

                    captures.push(CaptureRange::Single(capture_str));
                    if match_segments(segments, si + 1, path_segs, pi + 1, captures) {
                        return true;
                    }
                    captures.pop();
                }
            }
            false
        }
        Segment::StarDotStar => {
            if pi < path_segs.len() {
                let current_seg = path_segs[pi];
                // 对于 *.*，以最后一个点作为分隔符拆分为 stem 和 ext 两个捕获组
                if let Some((stem, ext)) = current_seg.rsplit_once('.') {
                    captures.push(CaptureRange::Single(stem));
                    captures.push(CaptureRange::Single(ext));

                    if match_segments(segments, si + 1, path_segs, pi + 1, captures) {
                        return true;
                    }

                    captures.pop();
                    captures.pop();
                }
            }
            false
        }
        Segment::RecursiveWild => {
            let remaining = path_segs.len() - pi;
            for k in 0..=remaining {
                captures.push(CaptureRange::Recursive(&path_segs[pi..pi + k]));
                if match_segments(segments, si + 1, path_segs, pi + k, captures) {
                    return true;
                }
                captures.pop();
            }
            false
        }
    }
}

/// 用捕获段填充模板
fn fill_template(segments: &[Segment], captures: &[String]) -> String {
    let mut result = String::with_capacity(128); // 直接预分配容量，干掉中间 Vec 开销
    let mut cap_idx = 0;

    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            result.push('/');
        }
        match seg {
            Segment::Literal(lit) => result.push_str(lit),
            Segment::Wildcard { prefix, suffix } => {
                result.push_str(prefix);
                if let Some(cap) = captures.get(cap_idx) {
                    result.push_str(cap);
                    cap_idx += 1;
                }
                result.push_str(suffix);
            }
            Segment::StarDotStar => {
                // 特殊处理 *.*：连续消耗两个捕获组，并在中间强行插入 '.'
                if let Some(stem) = captures.get(cap_idx) {
                    result.push_str(stem);
                    cap_idx += 1;
                }
                result.push('.');
                if let Some(ext) = captures.get(cap_idx) {
                    result.push_str(ext);
                    cap_idx += 1;
                }
            }
            Segment::RecursiveWild => {
                if let Some(cap) = captures.get(cap_idx) {
                    if !cap.is_empty() {
                        result.push_str(cap);
                    } else if result.ends_with('/') {
                        // 抵消掉刚才上面加入的多余的 '/'，解决双斜杠 Bug
                        result.pop();
                    }
                    cap_idx += 1;
                }
            }
        }
    }

    result
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_insensitive_literal_match() {
        // 场景：宏已验证合法。测试基础字面量的大小写不敏感匹配
        let matcher = PatternMatcher::compile("Src/Main.rs");

        let caps = matcher.match_path("src/main.rs").unwrap();
        assert!(caps.is_empty()); // 没有通配符，捕获组为空

        assert!(matcher.match_path("src/main.rs.bak").is_none());
    }

    #[test]
    fn test_wildcard_boundary_and_empty_capture() {
        // 场景：宏已验证合法。测试单个 `*` 匹配空字符串的边界行为
        let matcher = PatternMatcher::compile("logs/err*");

        // 当 `*` 完美匹配空字符串时
        let caps_empty = matcher.match_path("logs/err").unwrap();
        assert_eq!(caps_empty, vec![""]);

        // 匹配正常非空字符串
        let caps_normal = matcher.match_path("logs/err_404").unwrap();
        assert_eq!(caps_normal, vec!["_404"]);
    }

    #[test]
    fn test_star_dot_star_multi_dots() {
        // 场景：宏已验证合法。测试 `*.*` 在面对带有点的文件名时的右分割（rsplit_once）策略
        let matcher = PatternMatcher::compile("assets/*.*");

        // 多个点时，应当以最后一个点切分出 stem 和 ext
        let caps = matcher.match_path("assets/archive.tar.gz").unwrap();
        assert_eq!(caps, vec!["archive.tar", "gz"]);
    }

    #[test]
    fn test_recursive_wildcard_levels() {
        // 场景：宏已验证合法（有且仅有一个 `**`）。测试其匹配 0 级、1 级、多级路径的表现
        let matcher = PatternMatcher::compile("static/**/index.html");

        // 匹配 0 级路径（即 `**` 为空段）
        let caps_zero = matcher.match_path("static/index.html").unwrap();
        assert_eq!(caps_zero, vec![""]);

        // 匹配 1 级路径
        let caps_one = matcher.match_path("static/assets/index.html").unwrap();
        assert_eq!(caps_one, vec!["assets"]);

        // 匹配多级路径
        let caps_multi = matcher
            .match_path("static/assets/js/chunks/index.html")
            .unwrap();
        assert_eq!(caps_multi, vec!["assets/js/chunks"]);
    }

    #[test]
    fn test_deep_backtracking_with_recursive() {
        // 场景：宏已验证合法。复杂回溯测试。
        // 输入路径带有多个可能让字面量段产生混淆的节点，测试回溯机制是否能正确“吐出”节点并重新定位
        let matcher = PatternMatcher::compile("a/**/b/c.txt");

        // 路径中出现了两个 `b`，第一个 `b` 应该被 `**` 吞掉，从而让后面的 `b/c.txt` 成功匹配
        let caps = matcher.match_path("a/b/x/b/c.txt").unwrap();
        assert_eq!(caps, vec!["b/x"]);
    }

    #[test]
    fn test_fill_template_with_empty_recursive() {
        // 场景：宏已验证 source 和 target 的 capture 数量完全一致。
        // 测试当 `**` 捕获为空时，模板渲染是否能完美消灭双斜杠 Bug。
        let matcher = PatternMatcher::compile("src/**/main.rs");
        let template = PatternTemplate::compile("target/**/main.rs");

        // 对应 `**` 捕获为空的情况
        let caps = matcher.match_path("src/main.rs").unwrap();
        assert_eq!(caps, vec![""]);

        // 填充模板
        let filled = template.fill(&caps);
        // 如果处理不当，容易变成 "target//main.rs"。这里验证它必须是平滑的单斜杠
        assert_eq!(filled, "target/main.rs");
    }

    #[test]
    fn test_comprehensive_composition() {
        // 场景：宏已验证合法（捕获组数量均为 3 且类型对称）
        let matcher = PatternMatcher::compile("api/**/v1/*/*.*");
        let template = PatternTemplate::compile("backup/**/v1/*/*.*");

        let path = "api/users/roles/v1/fetch/avatar.png";
        let caps = matcher.match_path(path).unwrap();

        // 验证捕获顺序和内容：
        // 1. `**` -> "users/roles"
        // 2. `*`  -> "fetch"
        // 3. `*.*` -> "avatar", "png" (由于包含 2 个 capture，因此合起来当前模式共 4 个 captures)
        assert_eq!(caps, vec!["users/roles", "fetch", "avatar", "png"]);

        // 注入目标模板
        let filled = template.fill(&caps);
        assert_eq!(filled, "backup/users/roles/v1/fetch/avatar.png");
    }

    #[test]
    fn test_utf8_char_boundary_safety() {
        // 1. 测试前缀带有中文等非 ASCII 字符时，字节切片是否安全，会不会因 char_boundary 而 panic
        let matcher = PatternMatcher::compile("目录/*.jpg");

        // 匹配包含多字节字符的路径
        let caps = matcher.match_path("目录/中文.jpg").unwrap();
        assert_eq!(caps, vec!["中文"]);

        // 2. 测试大小写不敏感转换时，由于特殊字符转换导致长度发生变化的边界（如德语 ß 变大写是 SS）
        let matcher_macro = PatternMatcher::compile("prefix_*/file.txt");
        let caps_macro = matcher_macro.match_path("PREFIX_ß/file.txt").unwrap();
        assert_eq!(caps_macro, vec!["ß"]);
    }

    #[test]
    fn test_multi_single_wildcard_backtracking() {
        // 测试多个单级通配符与相同字面量交织时，match_segments 的非贪婪精确匹配
        let matcher = PatternMatcher::compile("a/*/b/*.txt");

        // 路径中包含多个 'b'，测试是否能正确分配
        let caps = matcher.match_path("a/b/b/apple.txt").unwrap();
        assert_eq!(caps, vec!["b", "apple"]);
    }
}
