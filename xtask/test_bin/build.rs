fn main() -> anyhow::Result<()> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("Failed to get cargo metadata");

    let arch = std::env::var("TARGET").unwrap();
    let target_dir = metadata.target_directory.join(arch).join("release");

    println!("cargo:rustc-link-lib=dylib=text-hook");
    println!("cargo:rustc-link-lib=static=text-hook:text_hook.dll");
    println!("cargo:rustc-link-search=native={target_dir}");
    Ok(())
}
