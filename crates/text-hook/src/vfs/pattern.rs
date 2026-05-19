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
