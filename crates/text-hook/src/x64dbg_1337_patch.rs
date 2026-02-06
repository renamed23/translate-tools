use crate::debug;

mod patch_from_1337 {
    translate_macros::generate_patch_fn_from_1337!("assets/x64dbg_1337_patch" => pub fn apply);
}

/// 应用1337文件的补丁数据，会对指定的模块进行写入字节；
/// 目前还不支持恢复数据
pub fn apply() -> crate::Result<()> {
    debug!("Start 1337 patch...");
    patch_from_1337::apply()
}
