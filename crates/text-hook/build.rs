use anyhow::{anyhow, bail};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use translate_utils::jis0208::is_jis0208;

fn main() -> anyhow::Result<()> {
    generate_constant()?;
    generate_patch_data()?;
    generate_mapping_data()?;

    Ok(())
}

fn generate_constant() -> anyhow::Result<()> {
    let default_config_path = Path::new("constant_assets/default_config.json");
    let config_path = Path::new("assets/config.json");

    println!("cargo:rerun-if-changed={}", default_config_path.display());
    println!("cargo:rerun-if-changed={}", config_path.display());

    if !default_config_path.exists() {
        bail!("assets/default_config.json 不存在");
    }

    // 读取默认配置
    let default_config_content = fs::read_to_string(default_config_path)
        .map_err(|e| anyhow!("无法读取 {}: {}", default_config_path.display(), e))?;

    let default_config: HashMap<String, serde_json::Value> =
        serde_json::from_str(&default_config_content)
            .map_err(|e| anyhow!("解析 assets/default_config.json 失败: {e}"))?;

    // 读取用户配置（如果存在）
    let user_config: HashMap<String, serde_json::Value> = if config_path.exists() {
        let config_content = fs::read_to_string(config_path)
            .map_err(|e| anyhow!("无法读取 {}: {}", config_path.display(), e))?;
        serde_json::from_str(&config_content)
            .map_err(|e| anyhow!("解析 assets/config.json 失败: {e}"))?
    } else {
        HashMap::new()
    };

    let mut constant_lines = Vec::new();
    constant_lines.push("// 通过config.json自动生成".to_string());
    constant_lines.push("".to_string());

    for (key, default_value) in &default_config {
        // 解析默认配置中的类型和值
        let type_str = default_value
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("default_config.json 中字段 '{}' 缺少 type", key))?;

        let default_val = default_value
            .get("value")
            .ok_or_else(|| anyhow!("default_config.json 中字段 '{}' 缺少 value", key))?;

        // 检查用户配置中是否有对应的值
        let final_value = if let Some(user_val) = user_config.get(key) {
            user_val // 使用用户配置的值
        } else {
            default_val // 使用默认值
        };

        if let Some(s) = final_value.as_str() {
            constant_lines.push(format!("pub const {}: {} = \"{}\";", key, type_str, s));
        } else if let Some(n) = final_value.as_u64() {
            constant_lines.push(format!("pub const {}: {} = {};", key, type_str, n));
        } else if let Some(b) = final_value.as_bool() {
            constant_lines.push(format!("pub const {}: {} = {};", key, type_str, b));
        } else {
            bail!("不支持的类型或值格式: {} = {:?}", key, final_value);
        }
    }

    let out_path = Path::new("src/constant.rs");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(out_path)?;
    file.write_all(constant_lines.join("\n").as_bytes())?;

    Ok(())
}

fn generate_mapping_data() -> anyhow::Result<()> {
    if std::env::var("CARGO_FEATURE_SHIFT_BIN").is_ok() {
        println!("cargo:warning=已启用 feature `shift_bin`，跳过生成 mapping_data");
        return Ok(());
    }

    let mapping_path = Path::new("assets/mapping.json");
    println!("cargo:rerun-if-changed={}", mapping_path.display());

    if !mapping_path.exists() {
        bail!("assets/mapping.json 不存在");
    }

    let s = fs::read_to_string(mapping_path)
        .map_err(|e| anyhow!("无法读取 {}: {}", mapping_path.display(), e))?;

    let map: HashMap<String, String> =
        serde_json::from_str(&s).map_err(|e| anyhow!("解析 assets/mapping.json 失败: {e}"))?;

    if map.is_empty() {
        println!("cargo:warning=assets/mapping.json 为空（将生成空的 mapping.rs）");
    }

    let mut entries: Vec<(u16, u16, String, String)> = Vec::new();
    let mut seen_codes: HashSet<u16> = HashSet::new();

    for (k, v) in map.into_iter() {
        // 强制单字符
        if k.chars().count() != 1 {
            bail!("mapping.json 的键必须是单个字符，发现: {k:?}");
        }
        if v.chars().count() != 1 {
            bail!("mapping.json 的值必须是单个字符，发现: {v:?}");
        }

        let kc = k.chars().next().unwrap();
        let vc = v.chars().next().unwrap();

        // 使用 is_jis0208 判断 key 是否为 JIS0208（可被 Shift_JIS 编码）
        if !is_jis0208(kc) {
            bail!("mapping.json 键 '{kc}' 不是 JIS0208（不可被 Shift_JIS 编码），请修正");
        }

        // value 必须是 shift-jis 不兼容字符
        if is_jis0208(vc) {
            println!(
                "mapping.json 值 '{vc}' 是 JIS0208（可以被 Shift_JIS 编码），但应为 shift-jis 不兼容字符"
            );
        }

        // 将 key 编码为 Shift_JIS
        let (enc, _, had_errors) = encoding_rs::SHIFT_JIS.encode(&k);
        if had_errors {
            bail!("键 '{k}' 编码为 Shift_JIS 时出现错误");
        }
        if enc.len() != 2 {
            bail!("键 '{}' 编码为 Shift_JIS 后长度异常: {}", k, enc.len());
        }

        let key_code: u16 = ((enc[0] as u16) << 8) | (enc[1] as u16);

        if seen_codes.contains(&key_code) {
            bail!("发现重复的 Shift_JIS 编码 0x{key_code:04X} 对应多个键（请检查 mapping.json）");
        }
        seen_codes.insert(key_code);

        // value -> utf16 codepoint（仅支持 BMP）
        let val_u32 = vc as u32;
        if val_u32 > 0xFFFF {
            bail!("mapping.json 的值 '{vc}' 超过 BMP（>0xFFFF），目前不支持");
        }
        let val_code: u16 = val_u32 as u16;

        entries.push((key_code, val_code, k.clone(), v.clone()));
    }

    // 排序（按 key 的编码）
    entries.sort_by_key(|e| e.0);

    // 生成文件 (替换原来的 HashMap 输出)
    let out_path = Path::new("src/mapping/mapping_data.rs");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut out = String::new();
    out.push_str("// 自动生成的 MAPPING 数据（phf）\n");
    out.push_str("// key: Shift_JIS 编码 (u16), value: UTF-16 码点 (u16, BMP)\n");
    out.push_str("use phf::phf_map;\n\n");
    out.push_str("pub(super) static SJIS_PHF_MAP: phf::Map<u16, u16> = phf_map! {\n");

    for (kcode, vcode, kch, vch) in &entries {
        // 注意：为保险起见给字面量加上类型后缀 u16
        out.push_str(&format!(
            "    0x{kcode:04X}u16 => 0x{vcode:04X}u16, // '{kch}' -> '{vch}'\n"
        ));
    }

    out.push_str("};\n\n");

    let mut f = fs::File::create(out_path)?;
    f.write_all(out.as_bytes())?;

    Ok(())
}

fn generate_patch_data() -> anyhow::Result<()> {
    if std::env::var("CARGO_FEATURE_PATCH").is_err() {
        println!("cargo:warning=未启用 feature `patch`，跳过生成 patch_data");
        return Ok(());
    }

    let raw_dir = Path::new("assets/raw");
    let translated_dir = Path::new("assets/translated");
    let out_path = Path::new("src/patch/patch_data.rs");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/raw");
    println!("cargo:rerun-if-changed=assets/translated");

    let mut raw_files: Vec<_> = if raw_dir.exists() {
        fs::read_dir(raw_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.path())
            .collect()
    } else {
        Vec::new()
    };
    raw_files.sort_by_key(|p| p.file_name().map(|n| n.to_os_string()).unwrap_or_default());

    if raw_files.is_empty() {
        println!("cargo:warning=目录 {raw_dir:?} 中没有任何文件（将生成空的 PATCHES）");
    }

    struct FileEntry {
        translated_path: std::path::PathBuf,
        raw_filename: String,
        len: usize,
        sha: [u8; 32],
    }

    let mut files: Vec<FileEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut seen_keys = HashSet::new();

    for raw_path in raw_files {
        println!("cargo:rerun-if-changed={}", raw_path.display());
        let translated_path = translated_dir.join(raw_path.file_name().unwrap());
        println!("cargo:rerun-if-changed={}", translated_path.display());

        if !translated_path.exists() {
            warnings.push(format!(
                "缺少翻译文件（已跳过）: {}",
                translated_path.display()
            ));
            continue;
        }

        let raw_data = match fs::read(&raw_path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("无法读取原始文件 {}: {}", raw_path.display(), e));
                continue;
            }
        };

        let translated_data = match fs::read(&translated_path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!(
                    "无法读取翻译文件 {}: {}",
                    translated_path.display(),
                    e
                ));
                continue;
            }
        };

        if raw_data.len() != translated_data.len() {
            errors.push(format!(
                "字节长度不匹配: {} -> raw={} bytes, translated={} bytes",
                raw_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>"),
                raw_data.len(),
                translated_data.len()
            ));
            continue;
        }

        let mut hasher = Sha256::new();
        hasher.update(&raw_data);
        let sha_bytes = hasher.finalize();
        let mut sha_arr = [0u8; 32];
        sha_arr.copy_from_slice(&sha_bytes);

        // 检查是否有重复的原始文件
        let key = (sha_arr, raw_data.len());
        if seen_keys.contains(&key) {
            errors.push(format!(
                "发现重复的原始文件: {} (SHA256: {:02x?}, 长度: {})",
                raw_path.display(),
                sha_arr,
                raw_data.len()
            ));
            continue;
        }
        seen_keys.insert(key);

        // 获取原始文件名
        let raw_filename = raw_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        files.push(FileEntry {
            translated_path: translated_path.clone(),
            raw_filename,
            len: raw_data.len(),
            sha: sha_arr,
        });
    }

    if !warnings.is_empty() {
        for w in &warnings {
            println!("cargo:warning=警告: {w}");
        }
    }

    if !errors.is_empty() {
        for e in &errors {
            println!("cargo:warning=错误: {e}");
        }
        bail!("patch_data 生成失败：存在错误，已中止构建");
    }

    // --- 下面开始生成 phf 代码 ---
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // header
    let mut out = String::new();
    out.push_str("// 自动生成的补丁数据（phf 版本）\n");
    out.push_str("use phf::{phf_map, phf_set};\n");
    out.push_str("use std::sync::LazyLock;\n\n");

    // include_flate lines
    for (idx, item) in files.iter().enumerate() {
        let patch_name = format!("PATCH_{:04}", idx + 1);
        let rel = item.translated_path.to_string_lossy().replace('\\', "/");
        out.push_str(&format!(
            "include_flate::flate!(\n    static {}: [u8] from \"{}\"\n);\n\n",
            patch_name, rel
        ));
    }

    out.push_str(
        "pub(super) static PATCHES: phf::Map<&'static [u8], &LazyLock<Vec<u8>>> = phf_map! {\n",
    );
    for (idx, item) in files.iter().enumerate() {
        let sha_hex = item
            .sha
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let key = format!("{}:{}", sha_hex, item.len);
        let patch_name = format!("PATCH_{:04}", idx + 1);
        out.push_str(&format!("    b\"{}\" => &{},\n", key, patch_name));
    }
    out.push_str("};\n\n");

    let mut lens: Vec<usize> = files.iter().map(|f| f.len).collect();
    lens.sort_unstable();
    lens.dedup();
    out.push_str("pub(super) static LEN_FILTER: phf::Set<usize> = phf_set! {\n");
    for l in &lens {
        out.push_str(&format!("    {},\n", l));
    }
    out.push_str("};\n\n");

    // optional filenames map for debug_output
    out.push_str("#[cfg(feature = \"debug_output\")]\n");
    out.push_str(
        "pub(super) static FILENAMES: phf::Map<&'static [u8], &'static str> = phf_map! {\n",
    );
    for item in files.iter() {
        let sha_hex = item
            .sha
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let key = format!("{}:{}", sha_hex, item.len);
        out.push_str(&format!("    b\"{}\" => \"{}\",\n", key, item.raw_filename));
    }
    out.push_str("};\n\n");

    // 写入文件
    let mut f = fs::File::create(out_path)?;
    f.write_all(out.as_bytes())?;
    Ok(())
}
