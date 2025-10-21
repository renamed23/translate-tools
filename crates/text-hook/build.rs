fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // 用于过程宏，当assets和constant_assets里面的文件变化时，会重新生成
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=constant_assets");

    Ok(())
}
