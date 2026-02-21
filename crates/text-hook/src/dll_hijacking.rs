mod dll {
    // 从指定目录读取PE文件，根据导出符号生成导出函数以及def文件
    translate_macros::generated_exports_from_hijacked_dll!("assets/hijacked" => "assets/exports.def");
}

/// 加载被劫持的DLL，并获取导出函数的地址，用于转发。
///
/// # Safety
/// - 调用者必须保证不在 `DllMain` 中调用该函数。
/// - 调用者必须保证初始化阶段仅调用一次，且不会并发调用。
pub unsafe fn load_library() {
    unsafe { dll::load_library() };
}

/// 卸载被劫持的DLL，并将所有保存导出函数地址的变量清0，
///
/// # Safety
/// - 调用者必须保证仅在清理阶段调用，且不会并发调用。
/// - 调用前应保证 `load_library` 已执行且不再有线程使用被转发函数。
pub unsafe fn unload_library() {
    unsafe { dll::unload_library() };
}
