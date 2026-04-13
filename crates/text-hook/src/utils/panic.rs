cfg_if::cfg_if! {
    if #[cfg(feature = "enable_debug_output")] {
        /// 设置自定义 panic hook，仅在启用 `enable_debug_output` 特性时有效
        /// 这个 hook 会使用 `debug!` 宏记录详细的 panic 信息
        pub fn set_debug_panic_hook() {
            std::panic::set_hook(Box::new(|panic_info| {
                // 获取 panic 的位置信息
                use crate::debug;
                let location = panic_info.location().map_or_else(
                    || String::from("unknown location"),
                    |loc| format!("file: {}, line: {}", loc.file(), loc.line()),
                );

                // 获取 panic 的详细消息
                let payload = panic_info.payload();
                let message = if let Some(msg) = payload.downcast_ref::<&str>() {
                    *msg
                } else if let Some(msg) = payload.downcast_ref::<String>() {
                    msg.as_str()
                } else {
                    "cannot extract panic message"
                };

                // 使用 debug! 宏输出 panic 信息（使用英文）
                debug!(raw "=== RUST DLL PANIC CAUGHT ===");
                debug!(raw "Location: {}", location);
                debug!(raw "Reason: {}", message);

                // 如果有 panic 的完整信息，也输出
                debug!(raw "Full info: {}", panic_info);
                debug!(raw "=== PANIC END ===");
            }));
        }
    } else {
        /// 当未启用 "enable_debug_output" 特性时的空实现
        #[allow(dead_code)]
        pub fn set_debug_panic_hook() {
            // 不设置任何 panic hook，保持默认行为
        }

    }
}
