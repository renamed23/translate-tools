use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::encoding_type::EncodingType;

/// 字符映射结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    /// 替身字符的编码类型
    pub src_encoding: EncodingType,
    /// 字符映射（替身字符 -> 原字符）
    pub mapping: HashMap<char, char>,
}

/// ReplacementPool：管理替身池与一对一映射
#[derive(Debug, Serialize, Deserialize)]
pub struct ReplacementPool {
    encoding: EncodingType, // 编码类型
    pool: Vec<char>,        // 经验证过的候选替身
    #[serde(skip)]
    free: VecDeque<char>, // 可用的替身
    #[serde(skip)]
    orig_to_repl: HashMap<char, char>, // 原字符 -> 替身
    #[serde(skip)]
    repl_to_orig: HashMap<char, char>, // 替身 -> 原字符
}

impl ReplacementPool {
    /// 从 JSON 文件加载替身池
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(&path)
            .with_context(|| format!("读取替身池文件失败: {}", path.as_ref().display()))?;
        Self::from_string(&s)
    }

    /// 从 JSON 字符串加载替身池
    pub fn from_string(json_str: &str) -> Result<Self> {
        let mut pool: Self = serde_json::from_str(json_str).context("解析替身池 JSON 失败")?;

        // 验证池中的字符是否被编码支持
        let mut invalid_chars = Vec::new();
        for &ch in &pool.pool {
            if !pool.encoding.contains_char(ch) {
                invalid_chars.push(ch);
            }
        }

        if !invalid_chars.is_empty() {
            bail!(
                "替身池中包含不被编码 {} 支持的字符: {:?}",
                pool.encoding,
                invalid_chars
            );
        }

        // 初始化运行时状态
        pool.free = VecDeque::from(pool.pool.clone());
        pool.orig_to_repl.clear();
        pool.repl_to_orig.clear();

        Ok(pool)
    }

    /// 将替身池保存到文件
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)
            .with_context(|| format!("写入替身池文件失败: {}", path.as_ref().display()))?;
        Ok(())
    }

    /// 重置所有映射（保留池内容）
    pub fn reset(&mut self) {
        self.orig_to_repl.clear();
        self.repl_to_orig.clear();
        self.free.clear();
        self.free.extend(self.pool.iter().cloned());
    }

    /// 为 orig 分配或返回已有替身
    pub fn get(&mut self, orig: char) -> Result<char> {
        if let Some(&r) = self.orig_to_repl.get(&orig) {
            return Ok(r);
        }
        if let Some(candidate) = self.free.pop_front() {
            assert!(!self.repl_to_orig.contains_key(&candidate));

            self.orig_to_repl.insert(orig, candidate);
            self.repl_to_orig.insert(candidate, orig);
            return Ok(candidate);
        }

        Err(anyhow!("替身池已耗尽，无法为 '{orig}' 分配替身"))
    }

    /// 把字符串映射为指定编码兼容字符串，返回映射结果
    pub fn map_text(&mut self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len());

        for ch in text.chars() {
            // ASCII字符和已经在目标编码中的字符直接保留
            if self.encoding.contains_char(ch) {
                out.push(ch);
                continue;
            }
            // 其他字符需要替换
            let repl = self.get(ch)?;
            out.push(repl);
        }

        Ok(out)
    }

    /// 生成字符映射（替身字符 -> 原字符）
    pub fn generate_mapping(&self) -> Mapping {
        Mapping {
            src_encoding: self.encoding,
            mapping: self.repl_to_orig.clone(),
        }
    }

    /// 将字符映射直接写入目标路径（JSON）
    pub fn write_mapping_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mapping = self.generate_mapping();
        let json = serde_json::to_string_pretty(&mapping)?;
        std::fs::write(&path, json)
            .with_context(|| format!("写入字符映射文件失败: {}", path.as_ref().display()))?;
        Ok(())
    }

    pub fn encoding(&self) -> EncodingType {
        self.encoding
    }

    pub fn pool_chars(&self) -> &[char] {
        &self.pool
    }

    pub fn current_mapping(&self) -> &HashMap<char, char> {
        &self.orig_to_repl
    }
}

/// 替身池构建器
#[derive(Debug, Clone)]
pub struct PoolBuilder {
    encoding: EncodingType,
    pool: HashSet<char>,
}

impl PoolBuilder {
    pub fn new(encoding: EncodingType) -> Self {
        Self {
            encoding,
            pool: HashSet::new(),
        }
    }

    /// 生成指定编码的字符池
    pub fn generate_pool(&mut self) -> Result<()> {
        let ranges = self.encoding.suggested_ranges();

        for (start, end) in ranges {
            for code in start..=end {
                if let Some(ch) = std::char::from_u32(code) {
                    // 验证字符是否在目标编码中
                    if self.encoding.contains_char(ch) {
                        self.pool.insert(ch);
                    }
                }
            }
        }

        if self.pool.is_empty() {
            bail!("初始字符池为空");
        }

        Ok(())
    }

    /// 从文本内容中剔除已使用的字符
    pub fn exclude_used_chars(&mut self, text: &str) {
        for ch in text.chars() {
            self.pool.remove(&ch);
        }
    }

    /// 构建 ReplacementPool
    pub fn build(self) -> ReplacementPool {
        let mut pool_chars: Vec<char> = self.pool.into_iter().collect();
        // 按码点从大到小排序，优先使用不常用的字符
        pool_chars.sort();
        pool_chars.reverse();

        ReplacementPool {
            encoding: self.encoding,
            pool: pool_chars,
            free: VecDeque::new(), // 会在from_string中初始化
            orig_to_repl: HashMap::new(),
            repl_to_orig: HashMap::new(),
        }
    }

    /// 将替身池保存到文件（通过构建ReplacementPool然后序列化）
    pub fn write_to_path<P: AsRef<Path>>(self, path: P) -> Result<()> {
        let pool = self.build();
        pool.save_to_path(path)
    }

    /// 获取当前池大小
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// 检查池是否为空
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}
