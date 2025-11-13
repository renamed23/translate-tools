use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=constant_assets");

    // DLL劫持的时候，我们需要按照劫持的DLL的序号进行导出
    // 但是目前Rust并不支持在代码中指定导出序号，所以我们需要def表
    if std::env::var("CARGO_FEATURE_DLL_HIJACKING").is_ok() {
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let def_path = manifest_dir.join("assets").join("exports.def");
        println!("cargo:rerun-if-changed={}", def_path.display());
        println!("cargo:rustc-link-arg-cdylib=/DEF:{}", def_path.display());
    }

    Ok(())
}
