use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 翻译词典中单条结构（用于解析 JSON 数组）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransPair {
    pub key: String,
    pub value: String,
}

/// 注意，为了更好的翻译质量，后续项目不要使用这个结构体
/// 翻译字典包装，保留插入顺序（pairs），pairs 中不含重复 key（第一次出现保留）
#[derive(Clone, Debug, Default)]
pub struct TranslatedDict {
    map: HashMap<String, String>,
    pairs: Vec<TransPair>, // 按插入顺序且不含重复项（保留第一次出现）
}

impl TranslatedDict {
    /// 从 JSON 字符串解析（`[{"key":"...","value":"..."}, ...]`）
    /// 遍历 raw_pairs：如果 map 中没有该 key，则插入到 map 和 pairs；否则跳过（保留第一次出现）
    pub fn from_string(s: &str) -> Result<Self> {
        let raw_pairs: Vec<TransPair> =
            serde_json::from_str(s).context("解析 TranslatedDict JSON 字符串失败")?;

        Ok(Self::from_vec(raw_pairs))
    }

    /// 从指定路径的文件解析
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(&path)
            .with_context(|| format!("读取文件失败 {:?}", path.as_ref()))?;
        Self::from_string(&s)
    }

    /// 从 Vec<TransPair> 构造（按 vec 的顺序插入，遇到重复 key 则跳过后续，保留第一次出现）
    pub fn from_vec(input_pairs: Vec<TransPair>) -> Self {
        let mut map: HashMap<String, String> = HashMap::with_capacity(input_pairs.len());
        let mut pairs: Vec<TransPair> = Vec::with_capacity(input_pairs.len());

        for p in input_pairs.into_iter() {
            if !map.contains_key(&p.key) {
                map.insert(p.key.clone(), p.value.clone());
                pairs.push(p);
            }
        }

        Self { map, pairs }
    }

    /// 生成新的翻译字典，对每个 value 应用映射函数（保持 pairs 的顺序）
    pub fn generate_dict<F>(&self, mut mapper: F) -> Result<Self>
    where
        F: FnMut(&str) -> Result<String>,
    {
        let mut new_pairs: Vec<TransPair> = Vec::with_capacity(self.pairs.len());
        let mut new_map: HashMap<String, String> = HashMap::with_capacity(self.pairs.len());

        for p in &self.pairs {
            let mapped_value = mapper(&p.value)?;
            new_map.insert(p.key.clone(), mapped_value.clone());
            new_pairs.push(TransPair {
                key: p.key.clone(),
                value: mapped_value,
            });
        }

        Ok(Self {
            map: new_map,
            pairs: new_pairs,
        })
    }

    /// 将字典写入到指定路径（JSON 格式）
    /// 直接序列化 pairs（pairs 中不含重复项，且按顺序）
    pub fn write_dict<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.pairs)?;
        std::fs::write(&path, json)
            .with_context(|| format!("写入字典文件失败: {}", path.as_ref().display()))?;
        Ok(())
    }

    /// 尝试获取译文，找不到返回 None
    pub fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    /// 如果找到则返回译文，否则返回原文（owned String）
    pub fn translate_or_original(&self, key: &str) -> String {
        self.map
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// 插入或更新一条映射
    /// - 若 key 已存在（pairs 中已有唯一项），更新该项的 value（不会新增重复项）
    /// - 若 key 新增，则 append 到 pairs，并写入 map
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        if self.contains_key(&key) {
            let pos = self.pairs.iter().position(|p| p.key == key).unwrap();
            self.pairs[pos].value = value.clone();
        } else {
            self.pairs.push(TransPair {
                key: key.clone(),
                value: value.clone(),
            });
        }

        self.map.insert(key, value);
    }

    /// 是否包含某个 key
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// 获取翻译字典长度
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// 翻译字典是否为空
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// 获取按顺序保存的 pairs 的只读切片（用于外部遍历）
    pub fn pairs(&self) -> &[TransPair] {
        &self.pairs
    }
}
