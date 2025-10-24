mod dll {
    // 从指定目录读取PE文件，根据输出符号生成导出函数
    translate_macros::generated_exports_from_hijacked_dll!("assets/hijacked");
}

pub unsafe extern "system" fn load_library() {
    unsafe { dll::load_library() };
}

pub unsafe extern "system" fn unload_library() {
    unsafe { dll::unload_library() };
}
