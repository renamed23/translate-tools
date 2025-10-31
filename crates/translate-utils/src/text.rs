use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// 单条条目
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: Option<String>,
    pub message: String,
    /// 存储完整的 JSON 数据以保留未知字段
    pub raw_data: Value,
}

impl Item {
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            name: None,
            message: message.clone(),
            raw_data: serde_json::json!({
                "message": message
            }),
        }
    }

    pub fn with_name(name: impl Into<String>, message: impl Into<String>) -> Self {
        let name_str = name.into();
        let message = message.into();
        Self {
            name: Some(name_str.clone()),
            message: message.clone(),
            raw_data: serde_json::json!({
                "name": name_str,
                "message": message
            }),
        }
    }

    /// 从 JSON Value 创建 Item
    pub fn from_value(value: &Value) -> Result<Self> {
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Item 缺少 message 字段"))?
            .to_string();

        Ok(Self {
            name,
            message,
            raw_data: value.clone(),
        })
    }

    /// 将 Item 转换回 JSON Value，保留所有原始字段
    pub fn to_value(&self) -> Value {
        let mut data = self.raw_data.clone();

        // 更新 name 和 message 字段，确保它们是最新的
        if let Some(name) = &self.name {
            data["name"] = Value::String(name.clone());
        } else {
            let _ = data.as_object_mut().map(|obj| obj.remove("name"));
        }

        data["message"] = Value::String(self.message.clone());

        data
    }
}

/// 封装文本内容的数组
#[derive(Clone, Debug, Default)]
pub struct Text {
    pub items: Vec<Item>,
}

impl Text {
    /// 新建空容器
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// 从 JSON 字符串解析（期待 JSON array of objects）
    pub fn from_string(s: &str) -> Result<Self> {
        let values: Vec<Value> = serde_json::from_str(s).context("解析 Text 字符串失败")?;

        let mut items = Vec::new();
        for value in values {
            let item =
                Item::from_value(&value).with_context(|| format!("解析 Item 失败: {}", value))?;
            items.push(item);
        }

        Ok(Self { items })
    }

    /// 从文件路径读取并解析
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = fs::read_to_string(&path)
            .with_context(|| format!("读取文件失败 {:?}", path.as_ref()))?;
        Self::from_string(&s)
    }

    /// 序列化为 pretty JSON 字符串
    pub fn to_string(&self) -> Result<String> {
        let values: Vec<Value> = self.items.iter().map(|item| item.to_value()).collect();
        serde_json::to_string_pretty(&values).context("序列化 Text 失败")
    }

    /// 序列化并写入指定路径（覆盖）
    pub fn write_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let s = self.to_string()?;
        fs::write(&path, s).with_context(|| format!("写入 Text 失败 {:?}", path.as_ref()))?;
        Ok(())
    }

    /// 追加一个只有 message 的条目（name = None）
    pub fn add(&mut self, message: impl Into<String>) {
        let item = Item::new(message);
        self.items.push(item);
    }

    /// 追加一个带 name 的条目
    pub fn add_with_name(&mut self, name: impl Into<String>, message: impl Into<String>) {
        let item = Item::with_name(name, message);
        self.items.push(item);
    }

    /// 获取指定位置的内容文本
    pub fn get_message(&self, index: usize) -> Option<&String> {
        self.items.get(index).map(|i| &i.message)
    }

    /// 获取指定位置的 Item
    pub fn get(&self, index: usize) -> Option<&Item> {
        self.items.get(index)
    }

    /// 获取指定位置的可变 Item
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Item> {
        self.items.get_mut(index)
    }

    /// 根据 (name, message) 去重，保留首次出现的顺序
    ///
    /// name 为 None 与 Some("") 被视为不同（Option 区分）
    /// 注意：去重时会保留第一个出现的条目的所有原始数据
    pub fn dedup(&mut self) {
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(self.items.len());
        for item in self.items.drain(..) {
            let key = (item.name.clone(), item.message.clone());
            if seen.insert(key) {
                out.push(item);
            }
        }
        self.items = out;
    }

    /// 根据 message 去重，忽略 name，保留首次出现的顺序
    /// 注意：去重时会保留第一个出现的条目的所有原始数据
    pub fn dedup_by_message(&mut self) {
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(self.items.len());
        for item in self.items.drain(..) {
            if seen.insert(item.message.clone()) {
                out.push(item);
            }
        }
        self.items = out;
    }

    /// 返回当前条目数量
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 生成新的文本，对每个 value 应用映射函数（保持 pairs 的顺序）
    /// 注意：原始数据会被保留，只有 name 和 message 会被更新
    pub fn generate_text<F>(&self, mut mapper: F) -> Result<Self>
    where
        F: FnMut(&Option<String>, &String) -> Result<(Option<String>, String)>,
    {
        let mut items = Vec::with_capacity(self.len());

        for item in &self.items {
            let (new_name, new_message) = mapper(&item.name, &item.message)?;
            let mut new_item = item.clone();
            new_item.name = new_name;
            new_item.message = new_message;
            items.push(new_item);
        }

        Ok(Self { items })
    }

    /// 将序列添加到文本中
    pub fn add_text(&mut self, text: Self) {
        self.items.extend(text.items);
    }

    /// 获取所有字符的集合，应用指定的过滤器
    ///
    /// 遍历所有 `item` 的 `name` 和 `message`，对每个字符应用过滤器函数
    /// 如果过滤器返回 true，则保留该字符
    pub fn get_filtered_chars<F>(&self, filter: F) -> HashSet<char>
    where
        F: Fn(char) -> bool,
    {
        let mut chars = HashSet::new();

        for item in &self.items {
            // 处理 `message` 中的字符
            for ch in item.message.chars() {
                if filter(ch) {
                    chars.insert(ch);
                }
            }

            // 处理 name 中的字符（如果存在）
            if let Some(name) = &item.name {
                for ch in name.chars() {
                    if filter(ch) {
                        chars.insert(ch);
                    }
                }
            }
        }

        chars
    }

    /// 获取所有字符的集合（不应用过滤器）
    ///
    /// 内部调用 `get_filtered_chars` 并使用始终返回 true 的过滤器
    pub fn get_chars(&self) -> HashSet<char> {
        self.get_filtered_chars(|_| true)
    }
}
