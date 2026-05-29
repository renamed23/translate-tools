use std::collections::BTreeSet;

use anyhow::Context;

pub fn run_tests() -> anyhow::Result<()> {
    read_eq(
        "src_data/file_in_both.txt",
        "dst_data/file_in_both.txt",
        "fallback: 双方都存在 → 应重定向到 dst",
    )?;

    read_eq(
        "src_data/file_only_in_src.txt",
        "src_data/file_only_in_src.txt",
        "fallback: 仅 src 存在 → 应保持原路径",
    )?;

    read_eq(
        "src_data/file_only_in_dst.txt",
        "dst_data/file_only_in_dst.txt",
        "fallback: 仅 dst 存在 → 应重定向到 dst",
    )?;

    read_eq(
        "src_data/nested/file.txt",
        "dst_data/nested/file.txt",
        "fallback: 嵌套文件 → 应重定向到 dst",
    )?;

    read_eq(
        "dont_exists/file.txt",
        "force_data/file.txt",
        "force: 应强制重定向",
    )?;

    list_eq(
        "src_data",
        &[
            "file_in_both.txt",
            "file_only_in_src.txt",
            "file_only_in_dst.txt",
            "nested",
        ],
        "fallback 枚举: 应合并 src + dst 条目",
    )?;

    list_eq(
        "dont_exists",
        &["file.txt"],
        "force 枚举: 应返回 force_data 条目",
    )?;

    list_recursive_eq(
        "src_data",
        &[
            "file_in_both.txt",
            "file_only_in_src.txt",
            "file_only_in_dst.txt",
            "nested/file.txt",
        ],
        "fallback 递归枚举: 应合并 src + dst 全部条目",
    )?;
    Ok(())
}

fn read_eq(path: &str, expected: &str, desc: &str) -> anyhow::Result<()> {
    let full = format!("test_bin_assets/{path}");
    let content =
        std::fs::read_to_string(&full).with_context(|| format!("[{desc}] 读取文件失败: {full}"))?;

    if content.trim() != expected {
        anyhow::bail!("[{desc}] 内容不匹配: 期望 '{expected}', 实际 '{content}'");
    }

    Ok(())
}

fn list_eq(dir: &str, expected: &[&str], desc: &str) -> anyhow::Result<()> {
    let full = format!("test_bin_assets/{dir}");
    let entries: BTreeSet<String> = std::fs::read_dir(&full)
        .with_context(|| format!("[{desc}] 枚举目录失败: {full}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let expected_set: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();

    if entries != expected_set {
        anyhow::bail!(
            "[{desc}] 条目不匹配: 期望 {:?}, 实际 {:?}",
            expected_set,
            entries
        );
    }

    Ok(())
}

fn list_recursive_eq(dir: &str, expected: &[&str], desc: &str) -> anyhow::Result<()> {
    let base = std::path::PathBuf::from("test_bin_assets").join(dir);
    let entries = collect_files(&base)
        .with_context(|| format!("[{desc}] 递归枚举失败: {}", base.display()))?;

    let expected_set: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();

    if entries != expected_set {
        anyhow::bail!(
            "[{desc}] 条目不匹配: 期望 {:?}, 实际 {:?}",
            expected_set,
            entries
        );
    }

    Ok(())
}

fn collect_files(path: &std::path::Path) -> anyhow::Result<BTreeSet<String>> {
    let mut files = BTreeSet::new();
    collect_files_impl(path, path, &mut files)?;
    Ok(files)
}

fn collect_files_impl(
    base: &std::path::Path,
    dir: &std::path::Path,
    out: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        if path.is_dir() {
            collect_files_impl(base, &path, out)?;
        } else {
            out.insert(rel);
        }
    }
    Ok(())
}
