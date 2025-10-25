mod dll {
    // 从指定目录读取PE文件，根据导出符号生成导出函数
    translate_macros::generated_exports_from_hijacked_dll!("assets/hijacked");
}

/// 加载被劫持的DLL，并获取导出函数的地址，用于转发。
/// 绝对不要在`DllMain`中调用该函数，否则会发生死锁。
pub unsafe fn load_library() {
    unsafe { dll::load_library() };
}

/// 卸载被劫持的DLL，并将所有保存导出函数地址的变量清0，
/// 在`PROCESS_DETACH`时调用。
pub unsafe fn unload_library() {
    unsafe { dll::unload_library() };
}
