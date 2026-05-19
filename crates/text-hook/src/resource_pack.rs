use std::path::Path;

mod pack {
    cfg_select! {
        feature = "embed_resource_pack" => {
            translate_macros::generate_resource_pack!("assets/resource_pack", "assets/config.json");
        }
        _ => {
            translate_macros::generate_resource_pack!(
                "assets/resource_pack",
                "assets/config.json",
                "assets/dist"
            );
        }
    }
}

/// 解压资源包到临时目录
pub fn extract() -> crate::Result<()> {
    pack::extract()
}

/// 清理资源包解压产生的临时文件
pub fn cleanup() -> crate::Result<()> {
    pack::cleanup()
}

/// 获取资源包临时目录
pub fn get_temp_dir() -> &'static Path {
    pack::get_temp_dir()
}
