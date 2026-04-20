use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{LazyLock, RwLock},
};

use ron::ser::PrettyConfig;
use serde::{Serialize, de::DeserializeOwned};

use crate::constant::STORAGE_PATH;

struct Storage {
    data: HashMap<String, String>,
    dirty: bool,
}

static STORAGE: LazyLock<RwLock<Storage>> = LazyLock::new(|| {
    let path = PathBuf::from(STORAGE_PATH);

    let data = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match ron::from_str(&content) {
                Ok(parsed) => parsed,
                Err(e) => {
                    crate::debug!("RON parse failed: {e}, falling back to empty storage");
                    HashMap::new()
                }
            },
            Err(e) => {
                crate::debug!("Read storage file failed: {e}, falling back to empty storage");
                HashMap::new()
            }
        }
    } else {
        crate::debug!("No storage file found, falling back to empty storage");
        HashMap::new()
    };

    RwLock::new(Storage { data, dirty: false })
});

/// 从存储中获取值，如果出现错误返回None
pub fn get_value<T: DeserializeOwned>(key: &str) -> Option<T> {
    let lock = STORAGE.read().expect("Storage lock poisoned");
    lock.data.get(key).and_then(|v| match ron::from_str(v) {
        Ok(v) => Some(v),
        Err(e) => {
            crate::debug!("Ron failed to load {key} with error: {e}");
            None
        }
    })
}

/// 将值插入存储中，如果存在旧值则被覆盖
pub fn set_value<T: Serialize>(key: &str, value: &T) -> crate::Result<()> {
    if key.is_empty() {
        crate::bail!("Storage key cannot be empty");
    }

    let ron_value =
        ron::to_string(value).map_err(|e| crate::anyhow!("Serialization failed: {e}"))?;

    let mut lock = STORAGE.write().expect("Storage lock poisoned");
    lock.data.insert(key.to_string(), ron_value);
    lock.dirty = true;

    Ok(())
}

/// 将内存中的更改同步至磁盘
pub fn flush() -> crate::Result<()> {
    let mut lock = STORAGE.write().expect("Storage lock poisoned");

    if !lock.dirty {
        return Ok(());
    }

    let config = ron::ser::to_string_pretty(&lock.data, PrettyConfig::default())
        .map_err(|e| crate::anyhow!("RON serialization error: {e}"))?;

    fs::write(STORAGE_PATH, config).map_err(|e| crate::anyhow!("File write error: {e}"))?;
    lock.dirty = false;

    Ok(())
}

/// 删除指定值
#[allow(dead_code)]
pub fn remove(key: &str) {
    let mut lock = STORAGE.write().expect("Storage lock poisoned");
    if lock.data.remove(key).is_some() {
        lock.dirty = true;
    }
}
