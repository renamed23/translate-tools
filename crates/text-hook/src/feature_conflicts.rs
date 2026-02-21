#[cfg(all(
    feature = "apply_1337_patch_on_attach",
    feature = "apply_1337_patch_on_hwbp_hit"
))]
compile_error!(
    "特性 `apply_1337_patch_on_attach` 和 `apply_1337_patch_on_hwbp_hit` 不能同时启用，因为它们都涉及对同一补丁的应用时机控制，可能导致冲突和不确定行为。请根据需要选择一个特性启用。"
);
