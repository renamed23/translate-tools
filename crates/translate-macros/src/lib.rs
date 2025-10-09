pub(crate) mod byte_slice;
pub(crate) mod ffi_catch_unwind;
pub(crate) mod flate;
pub(crate) mod generate_detours;
pub(crate) mod utils;

use proc_macro::TokenStream;

/// 将十六进制字节字符串转换为字节数组字面量
///
/// 此宏接受一个由空格分隔的十六进制字节字符串，将其转换为对应的字节数组字面量。
/// 主要用于在编译时将十六进制序列转换为类型安全的字节数组。
///
/// # 语法
///
/// ```rust
/// byte_slice!("0C 00 0E 00 90 7F AC")
/// ```
///
/// 上述调用将生成：
/// ```rust
/// [0x0Cu8, 0x00u8, 0x0Eu8, 0x00u8, 0x90u8, 0x7Fu8, 0xACu8]
/// ```
///
/// # 参数要求
///
/// - **输入格式**：必须是一个字符串字面量，包含由**单个空格**分隔的两位十六进制数
/// - **字符限制**：只能包含 `0-9`、`A-F`、`a-f` 和空格字符
/// - **长度要求**：每个字节必须恰好为2个字符（前导零不能省略）
/// - **边界限制**：字符串不能以空格开头或结尾
///
/// # 示例用法
///
/// ```rust
/// use translate_macros::byte_slice;
///
/// // 基本用法
/// const BYTE_ARRAY: [u8; 4] = byte_slice!("48 65 6C 6C");
/// assert_eq!(BYTE_ARRAY, [0x48, 0x65, 0x6C, 0x6C]);
///
/// // 在模式匹配中使用
/// match some_byte {
///     byte_slice!("FF") => println!("匹配到 0xFF"),
///     _ => println!("其他值"),
/// }
/// ```
///
/// # 生成代码
///
/// 宏生成的代码是类型安全的，每个字节都明确标记为 `u8` 类型：
/// ```rust
/// // 输入：byte_slice!("0C 00 FF")
/// // 输出：
/// [0x0Cu8, 0x00u8, 0xFFu8]
/// ```
///
/// 这使得结果可以直接用于需要 `[u8; N]` 类型的上下文。
#[proc_macro]
pub fn byte_slice(input: TokenStream) -> TokenStream {
    byte_slice::byte_slice(input)
}

/// 标记属性：`#[detour(...)]`
///
/// 将某个 trait 方法标记为“需要为其生成导出 wrapper 与 detour 静态”的元数据属性。
/// 此属性本身为 no-op（不修改被标注项），仅作为元数据供 `#[generate_detours]` 读取与处理。
///
/// # 语法
///
/// ```rust
/// #[detour(
///     dll = "gdi32.dll",                              // 必需，目标动态库名（字符串字面量）
///     symbol = "TextOutA",                            // 必需，目标导出符号名（字符串字面量）
///     export = "text_out",                            // 可选，生成的 wrapper 导出名（字符串字面量），默认使用 trait 方法名
///     fallback = "winapi::shared::minwindef::FALSE"   // 可选，捕获 panic/unwind 时的回退值（字符串字面量，内部会解析为 Rust 表达式）
///     calling_convention = "system"                   // 可选，调用约定（字符串字面量），默认 "system"
/// )]
/// unsafe fn text_out(&self, hdc: HDC, x: c_int, y: c_int, lp: LPCSTR, c: c_int) -> BOOL;
/// ```
///
/// # 字段说明
///
/// * `dll`：**必需**。目标模块名称（字符串字面量），用于运行时查找符号地址，例如 `"gdi32.dll"`。
/// * `symbol`：**必需**。目标导出符号名（字符串字面量），例如 `"TextOutA"`。
/// * `export`：可选。生成的 wrapper 导出名（字符串字面量）。若省略，宏将使用 trait 方法名作为导出名。
/// * `fallback`：可选。字符串字面量，内容将被解析为 Rust 表达式作为 wrapper 在捕获 panic/unwind 时的返回值。
///   建议显式提供 `fallback`；若不提供，宏默认用 `Default::default()`，但当返回类型不实现 `Default` 时会导致编译错误。
#[proc_macro_attribute]
pub fn detour(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// 在 trait 上自动生成 detour：`#[generate_detours]`
///
/// 应用于 trait 定义。该宏遍历 trait 中的每个方法，对于带有 `#[detour(...)]` 标记的 trait 方法，
/// 宏会基于方法签名自动生成两类项：
///
/// 1. 一个 `pub unsafe extern "system" fn <export_name>(...) -> Ret` 的 C-ABI 风格 wrapper（导出函数），
///    wrapper 内部通过 `crate::hook::hook_instance().<method>(...)` 转发到当前的 Hook 实现，并使用 `ffi_catch_unwind`
///    或等价保护来在 panic/unwind 时返回 `fallback` 指定的值；
/// 2. 一个名为 `HOOK_<METHOD_UPPER>` 的 `pub static` 变量，类型为
///    `once_cell::sync::Lazy<retour::GenericDetour<unsafe extern "system" fn(...) -> Ret>>`，
///    该静态在首次访问时会查找 `dll` 的 `symbol` 地址并尝试注册 detour（使用 `retour::GenericDetour::new`）。
///
/// `generate_detours` 只负责生成 wrapper 与 detour 静态；它**不**自动触发静态初始化（即不会自动在 crate 初始化时启用 detour）。
/// 若需要在程序启动时启用 detour，请在适当时机主动引用对应的 `HOOK_<NAME>` 静态或显式触发初始化。
///
/// # 用法
///
/// ```rust
/// use detour_gen::generate_detours;
/// use detour_gen::detour;
///
/// #[generate_detours]
/// pub trait Hook: Send + Sync + 'static {
///     #[detour(
///         dll = "gdi32.dll",                              // 必需，目标动态库名（字符串字面量）
///         symbol = "TextOutA",                            // 必需，目标导出符号名（字符串字面量）
///         export = "text_out",                            // 可选，生成的 wrapper 导出名（字符串字面量），默认使用 trait 方法名
///         fallback = "winapi::shared::minwindef::FALSE"   // 可选，捕获 panic/unwind 时的回退值（字符串字面量，内部会解析为 Rust 表达式）
///         calling_convention = "system"                   // 可选，调用约定（字符串字面量），默认 "system"
///     )]
///     unsafe fn text_out(&self, hdc: HDC, x: c_int, y: c_int, lp: LPCSTR, c: c_int) -> BOOL;
///
///     // 未标注 detour 的方法不会生成 wrapper / static
///     fn font_face(&self) -> &'static str;
/// }
/// ```
#[proc_macro_attribute]
pub fn generate_detours(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_detours::generate_detours(attr, item)
}

/// 为 FFI 导出的函数自动生成 panic 捕获包装的属性宏实现。
///
/// # 用途
/// 将这个属性应用到 `fn` 上后，函数体会被自动用 `std::panic::catch_unwind` 包裹，
/// 当函数内部发生 panic 时不会让 panic 穿出 FFI 边界，而是返回用户在属性中指定的回退值（fallback）。
///
/// # 属性语法
/// - `#[ffi_catch_unwind]`：不带参数时默认回退值为 `()`（空元组）。
/// - `#[ffi_catch_unwind(<fallback_expr>)]`：带一个表达式作为回退值，例如 `#[ffi_catch_unwind(FALSE)]`、`#[ffi_catch_unwind(0)]`、`#[ffi_catch_unwind(())]` 等。
///
/// 回退值表达式必须与被修饰函数的返回类型兼容（可隐式或显式转换通过类型检查）。
///
/// # 限制与注意事项
/// - 这是一个属性宏（`#[proc_macro_attribute]`），只能应用于项（`fn`），不能用于任意表达式或局部块。
/// - 属性宏需定义在 `proc-macro` crate 中并作为依赖引入，被修饰函数通常位于另一个 crate（proc macros 不能在同一 crate 内定义并使用）。
/// - 请确保回退值类型与函数返回类型兼容，否则会在编译期报错（这实际上是安全检查的一部分）。
/// - 保留并不改变函数签名（`extern "C"` / `extern "system"` / `#[no_mangle]` 等仍然有效）。
///
/// # 示例
/// ```rust
/// // 在 proc-macro crate 中定义后，在使用处：
/// #[ffi_catch_unwind(FALSE)]
/// #[no_mangle]
/// pub unsafe extern "system" fn DllMain(
///     _hinst_dll: HMODULE,
///     fdw_reason: DWORD,
///     _lpv_reserved: LPVOID,
/// ) -> BOOL {
///     // 原始函数体保持不变；若内部 panic 则返回 FALSE
///     const PROCESS_ATTACH: DWORD = 1;
///     if fdw_reason == PROCESS_ATTACH {
///         crate::panic_utils::set_debug_panic_hook();
///         crate::hook::set_hook_instance(Box::new(DefaultHook));
///
///         #[cfg(feature = "custom_font")]
///         crate::custom_font::add_font();
///
///         crate::hook::enable_text_hooks();
///     }
///
///     TRUE
/// }
/// ```
#[proc_macro_attribute]
pub fn ffi_catch_unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    ffi_catch_unwind::ffi_catch_unwind(attr, item)
}

/// 将文件在编译时压缩并嵌入为静态变量，运行时解压访问。
///
/// # 语法
/// ```ignore
/// flate!([pub] static VARIABLE_NAME: [u8] from "file_path");
/// ```
///
/// # 参数说明
/// - `[pub]`: 可选，如果提供则生成公有的静态变量
/// - `VARIABLE_NAME`: 静态变量的标识符
/// - `[u8]`: 类型标记（实际类型为 `LazyLock<Vec<u8>>`）
/// - `"file_path"`: 相对于 `CARGO_MANIFEST_DIR` 的文件路径
///
/// # 返回值类型
/// 生成的静态变量类型为 `LazyLock<Vec<u8>>`，在首次访问时自动解压数据。
///
/// # 特性
/// - **编译时压缩**: 使用 zstd 算法（级别 0）在编译时压缩文件
/// - **运行时解压**: 数据在首次访问时解压，避免启动时性能开销
/// - **路径解析**: 文件路径相对于项目根目录（`CARGO_MANIFEST_DIR`）
/// - **错误处理**: 编译时检查文件存在性和可读性
///
/// # 示例
/// ```
/// // 在 crate root 或 mod 中
/// use your_crate::flate;
///
/// // 嵌入并压缩配置文件
/// flate!(static CONFIG_DATA: [u8] from "config/app.toml");
///
/// // 公有的嵌入资源
/// flate!(pub static ASSET_DATA: [u8] from "assets/image.png");
///
/// // 使用时
/// fn use_embedded_data() {
///     let data = &*CONFIG_DATA; // 首次访问时解压
///     println!("Config size: {}", data.len());
/// }
/// ```
///
/// # 注意事项
/// - 文件路径相对于 `CARGO_MANIFEST_DIR`（项目根目录）
/// - 压缩级别固定为 0（快速压缩）
/// - 需要运行时解压函数 `crate::patch::decompress_zstd` 的支持
/// - 生成的静态变量是 `LazyLock<Vec<u8>>` 类型，需要通过 `&*VAR` 访问数据
#[proc_macro]
pub fn flate(input: TokenStream) -> TokenStream {
    flate::flate(input)
}
