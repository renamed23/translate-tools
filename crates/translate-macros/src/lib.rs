#[allow(unused_macros)]
macro_rules! syn_bail {
    // 使用 token 的 span（自动提取）
    ($token:expr, $($arg:tt)*) => {
        return Err(syn::Error::new_spanned(
            $token,
            format!($($arg)*)
        ))
    };
}

#[allow(unused_macros)]
macro_rules! syn_bail2 {
    // 使用 call_site span（宏调用位置）
    ($($arg:tt)*) => {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!($($arg)*)
        ))
    };
}

#[allow(unused_macros)]
macro_rules! syn_err {
    // 使用 token 的 span（自动提取）
    ($token:expr, $($arg:tt)*) => {
       syn::Error::new_spanned(
            $token,
            format!($($arg)*)
        )
    };
}

#[allow(unused_macros)]
macro_rules! syn_err2 {
    // 使用 call_site span（宏调用位置）
    ($($arg:tt)*) => {
     syn::Error::new(
            proc_macro2::Span::call_site(),
            format!($($arg)*)
        )
    };
}

pub(crate) mod impls;
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
    match impls::byte_slice::byte_slice(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 在 trait 上自动生成 detour：`#[detour_trait]`
///
/// 应用于 trait 定义。该宏遍历 trait 中的每个方法，对于带有 `#[detour(...)]` 标记的 trait 方法，
/// 宏会基于方法签名自动生成两类项：
///
/// 1. 一个 `pub unsafe extern "system" fn <wrapper_name>(...) -> Ret` 的 C-ABI 风格 wrapper，
///    wrapper 内部通过 `crate::hook::impls::HookImplType::<method>(...)` 转发到当前的 Hook 实现，并使用 `ffi_guard`
///    或等价保护在 panic/unwind 时返回 `fallback` 指定的值；若内部返回 `Err(...)`，则自动回退到
///    `crate::call!(HOOK_<METHOD>, ...)` 调用原始 HOOK。
///
///    - 若 `detour` 属性中提供了 `export = "..."`，则该值会作为生成 wrapper 的 **Rust 函数名**；
///    - 若未提供，则默认使用 trait 方法名；
///    - 只有启用 `feature = "export_hook_symbols"` 时，wrapper 才会额外带上 `#[no_mangle]`，从而以未改名符号导出；
/// 2. 一个名为 `HOOK_<METHOD_UPPER>` 的 `pub static` 变量，类型为
///    `std::sync::LazyLock<retour::GenericDetour<unsafe extern "system" fn(...) -> Ret>>`，
///    该静态在首次访问时会查找 `dll` 的 `symbol` 地址并尝试注册 detour（使用 `retour::GenericDetour::new`）。
///
/// `detour_trait` 只负责生成 wrapper 与 detour 静态；它**不**自动触发静态初始化（即不会自动在 crate 初始化时启用 detour）。
/// 若需要在程序启动时启用 detour，请在适当时机主动引用对应的 `HOOK_<NAME>` 静态或显式触发初始化。
///
/// # 用法
///
/// ```rust
/// #[detour_trait]
/// pub trait Hook: Send + Sync + 'static {
///     #[detour(
///         dll = "gdi32.dll",                              // 必需，目标动态库名（字符串字面量）
///         symbol = "TextOutA",                            // 必需，目标导出符号名（字符串字面量）
///         export = "text_out",                            // 可选，生成的 wrapper 的 Rust 函数名
///         fallback = "FALSE"                              // 可选，仅在 panic/unwind 时使用的回退值（字符串字面量，内部会解析为 Rust 表达式）
///         calling_convention = "system"                   // 可选，调用约定（字符串字面量），默认 "system"
///     )]
///     unsafe fn text_out(hdc: HDC, x: c_int, y: c_int, lp: LPCSTR, c: c_int) -> BOOL;
///
///     // 未标注 detour 的方法不会生成 wrapper / static
///     fn font_face() -> &'static str;
/// }
/// ```
///
/// 对于带有 `#[detour(...)]` 的 trait 方法，如果你只写声明而省略函数体：
///
/// ```rust
/// #[detour_trait]
/// pub trait ExitProcess {
///     #[detour(dll = "kernel32.dll", symbol = "ExitProcess")]
///     unsafe fn exit_process(u_exit_code: u32);
/// }
/// ```
///
/// 宏会自动把它补成一个默认实现：
///
/// ```rust,ignore
/// unsafe fn exit_process(u_exit_code: u32) {
///     unimplemented!()
/// }
/// ```
///
/// 注意：这个默认实现只是为了让 trait 接口书写更简洁；实际生成的 detour wrapper 仍然会调用
/// `crate::hook::impls::HookImplType::<method>(...)`，不会把这个 `unimplemented!()` 当作运行时 fallback。
#[proc_macro_attribute]
pub fn detour_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    match impls::detour::detour_trait::detour_trait(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 为函数自动生成detour：`#[detour_fn(...)]`
///
/// # 语法
///
/// ```rust
/// #[detour_fn(
///     dll = "gdi32.dll",                              // 必需，目标动态库名（字符串字面量）
///     symbol = "TextOutA",                            // 必需，目标导出符号名（字符串字面量）
///     fallback = "FALSE"                              // 可选，仅在 panic/unwind 时使用的回退值（字符串字面量，内部会解析为 Rust 表达式）
/// )]
/// unsafe extern "system" fn text_out(hdc: HDC, x: c_int, y: c_int, lp: LPCSTR, c: c_int) -> BOOL;
/// ```
///
/// # 字段说明
///
/// * `dll`：**必需**。目标模块名称（字符串字面量），用于运行时查找符号地址，例如 `"gdi32.dll"`。
/// * `symbol`：**必需**。目标导出符号名（字符串字面量），例如 `"TextOutA"`。
/// * `fallback`：可选。字符串字面量，内容将被解析为 Rust 表达式作为 wrapper 在捕获 panic/unwind 时的返回值。
///   若函数内部返回 `Err(...)`，则会自动回退到 `crate::call!(HOOK_<FN>, ...)` 调用原始 HOOK。
///   建议显式提供 `fallback`；若不提供，宏默认用 `Default::default()`，但当返回类型不实现 `Default` 时会导致编译错误。
#[proc_macro_attribute]
pub fn detour_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    match impls::detour::detour_fn::detour_fn(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 为 FFI 函数生成 panic / error 边界保护。
///
/// 该属性会把目标函数体包裹进保护逻辑中：
/// - 当启用 `panic = "unwind"` 时，使用 `std::panic::catch_unwind` 捕获 panic，防止 panic 穿过 FFI 边界；
/// - 当函数返回 `Result<T, E>` 时，会自动把返回签名拍平为 `T`，并在出现 `Err(...)` 时返回用户指定的兜底值。
///
/// # 属性参数
///
/// 支持以下键值对：
///
/// - `on_panic = <expr>`：**必需**。当函数内部 panic 时返回的值。
/// - `on_err = <expr>`：可选。仅当函数返回 `Result<T, E>` 时有意义；当返回 `Err(...)` 时使用该值。
/// - `on_err_or_panic = <expr>`：可选简写，同时作为 `on_panic` 与 `on_err` 的值。
///
/// 如果函数返回 `Result<T, E>`，则必须提供 `on_err` 或 `on_err_or_panic`。
///
/// # 行为说明
///
/// - 对普通返回值函数：仅拦截 panic，函数签名保持不变；
/// - 对 `Result<T, E>` 函数：
///   - 生成后的函数签名会从 `-> Result<T, E>` 变为 `-> T`；
///   - `Ok(value)` 会被直接返回；
///   - `Err(err)` 会记录调试日志并返回 `on_err` 指定的值；
///   - panic 时返回 `on_panic` 指定的值。
///
/// # 限制与注意事项
///
/// - 这是属性宏（`#[proc_macro_attribute]`），只能用于函数项。
/// - 请确保 `on_panic` / `on_err` 表达式与最终函数返回类型兼容，否则会在编译期报错。
/// - 宏不会移除或修改原函数上的 `extern "C"` / `extern "system"` / `#[no_mangle]` 等 ABI 相关声明。
///
/// # 示例
///
/// ## 普通返回值函数
/// ```rust
/// #[ffi_guard(on_panic = FALSE)]
/// #[no_mangle]
/// pub unsafe extern "system" fn DllMain(
///     _hinst_dll: HMODULE,
///     fdw_reason: DWORD,
///     _lpv_reserved: LPVOID,
/// ) -> BOOL {
///     // 原始函数体保持不变；若内部 panic 则返回 FALSE
///     const PROCESS_ATTACH: DWORD = 1;
///     if fdw_reason == PROCESS_ATTACH {
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
///
/// ## 返回 `Result` 的函数
/// ```rust
/// #[ffi_guard(on_err_or_panic = 0)]
/// pub unsafe extern "system" fn do_work(arg: i32) -> crate::Result<i32> {
///     if arg < 0 {
///         crate::bail!("invalid arg");
///     }
///     Ok(arg + 1)
/// }
///
/// // 展开后等价于返回 `i32`，发生 Err 或 panic 时返回 0。
/// ```
#[proc_macro_attribute]
pub fn ffi_guard(attr: TokenStream, item: TokenStream) -> TokenStream {
    match impls::ffi_guard::ffi_guard(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 将文件或目录中的单个文件在编译时嵌入为静态变量或常量。
///
/// # 语法
/// ```ignore
/// embed!([pub] static VARIABLE_NAME: [u8] from "path");
/// embed!([pub] const VARIABLE_NAME: [u8] from "path");
/// ```
///
/// # 参数说明
/// - `[pub]`: 可选，如果提供则生成公有的变量
/// - `static` 或 `const`: 指定嵌入模式
///   - `static`: 编译时压缩，运行时通过 `LazyLock` 解压访问
///   - `const`: 不压缩，直接嵌入原始字节
/// - `VARIABLE_NAME`: 变量标识符
/// - `[u8]`: 类型标记（`static` 实际类型为 `LazyLock<Vec<u8>>`，`const` 实际类型为 `&[u8]`）
/// - `"path"`: 相对于 `CARGO_MANIFEST_DIR` 的文件路径或目录路径
///
/// # 路径处理规则
/// - 如果指定的是文件路径，则直接使用该文件
/// - 如果指定的是目录路径，则检查目录中是否只有一个文件：
///   - 如果只有一个文件，自动使用该文件
///   - 如果没有文件或多个文件，编译时报错
///
/// # 返回值类型
/// - **`static` 模式**: `LazyLock<Vec<u8>>`，在首次访问时自动解压数据
/// - **`const` 模式**: `&[u8]`，直接访问原始字节，无运行时开销
///
/// # 示例
/// ```
/// // 在 crate root 或 mod 中
/// use your_crate::embed;
///
/// // static 模式：编译时压缩，运行时解压
/// embed!(static CONFIG_DATA: [u8] from "config/app.toml");
///
/// // const 模式：直接嵌入，无压缩
/// embed!(const SMALL_ICON: [u8] from "assets/icon.png");
///
/// // 嵌入目录中的单个文件（当目录中只有一个文件时）
/// embed!(static ASSET_DATA: [u8] from "assets/single_file_dir");
///
/// // 公有的嵌入资源
/// embed!(pub static IMAGE_DATA: [u8] from "images/logo.png");
/// embed!(pub const PUBLIC_KEY: [u8] from "keys/public.pem");
///
/// // 使用时
/// fn use_embedded_data() {
///     // static 模式：通过 &*VAR 访问，首次访问时解压
///     let config = &*CONFIG_DATA;
///     println!("Config size: {}", config.len());
///
///     // const 模式：直接访问
///     let icon = SMALL_ICON;
///     println!("Icon size: {}", icon.len());
/// }
/// ```
///
/// # 注意事项
/// - 路径相对于 `CARGO_MANIFEST_DIR`（项目根目录）
/// - 目录路径必须包含且仅包含一个文件，否则编译失败
#[proc_macro]
pub fn embed(input: TokenStream) -> TokenStream {
    match impls::embed::embed(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 一个过程宏，用于自动搜索并生成条件编译的钩子实现类型别名。
///
/// 这个宏会扫描指定目录下的 Rust 文件，查找符合命名规范的钩子结构体，
/// 然后为每个找到的结构体生成一个条件编译的类型别名。
///
/// # 语法
/// ```ignore
/// search_hook_impls!("relative/path/to/hook/implementations" => [pub] type AliasName);
/// ```
///
/// # 文件处理规则
/// - 只处理 `.rs` 扩展名的文件
/// - 自动跳过 `mod.rs` 和 `lib.rs` 文件
/// - 对于每个文件 `example.rs`，期望找到名为 `ExampleHook` 的结构体
/// - 文件名转换为大驼峰后加上 "Hook" 后缀作为期望的结构体名
///
/// # 生成代码示例
/// 假设输入：
/// ```ignore
/// search_hook_impls!("src/hooks" => pub type HookImpl);
/// ```
///
/// 目录 `src/hooks` 下有 `user_auth.rs` 和 `data_validation.rs` 文件，
/// 且这些文件中分别有 `UserAuthHook` 和 `DataValidationHook` 结构体，
/// 将生成：
/// ```ignore
/// #[cfg(feature = "user_auth")]
/// pub type HookImpl = user_auth::UserAuthHook;
///
/// #[cfg(feature = "data_validation")]
/// pub type HookImpl = data_validation::DataValidationHook;
/// ```
#[proc_macro]
pub fn search_hook_impls(input: TokenStream) -> TokenStream {
    match impls::search_hook_impls::search_hook_impls(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 从 JSON 配置文件生成 Rust 常量的过程宏
///
/// # 功能描述
/// 这个宏从两个 JSON 配置文件中读取配置项并生成对应的 Rust 常量：
/// - 默认配置文件：包含所有配置项的默认值和类型定义
/// - 用户配置文件：可以覆盖默认配置中的值
///
/// # 输入参数
/// 接受两个字符串字面量参数，用逗号分隔：
/// - `default_path`: 默认配置文件的相对路径（相对于 `CARGO_MANIFEST_DIR`）
/// - `user_path`: 用户配置文件的相对路径（相对于 `CARGO_MANIFEST_DIR`）
///
/// 支持的字段：
/// - `type`: Rust 类型标识符（如 `"&str"`, `"u32"`, `"bool"`, `"&[u16]"` 等）
/// - `value`: 常量的值，可以是字符串、数字、布尔值或数组
/// - `encode_to_u16`（可选）: 仅对字符串有效，为 `true` 时将字符串编码为 UTF-16 字节数组
///
/// # 生成规则
/// - 常量名：将配置键名中的非字母数字字符替换为下划线
/// - 类型：直接使用配置中的类型字符串
/// - 值：优先使用用户配置，不存在时使用默认配置
/// - 字符串处理：当 `encode_to_u16` 为 `true` 时，字符串会被转换为 `&[u16]` 数组
///
/// # 示例
/// ```
/// generate_constants_from_json!("config/default.json", "config/user.json");
/// ```
///
/// # 错误处理
/// - 文件读取失败：编译时错误
/// - JSON 解析失败：编译时错误
/// - 缺少必需字段（type/value）：编译时错误
/// - 类型解析失败：编译时错误
///
/// # 注意事项
/// - 配置文件路径相对于 `CARGO_MANIFEST_DIR`（项目根目录）
/// - 用户配置文件中不存在的配置项将使用默认值
/// - 用户配置文件中多余的配置项会被忽略
/// - 生成的常量都是 `pub const`
/// - 数组类型会生成为切片引用 `&[...]`
#[proc_macro]
pub fn generate_constants_from_json(input: TokenStream) -> TokenStream {
    match impls::generate_constants_from_json::generate_constants_from_json(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 生成字符映射数据的过程宏。
///
/// # 功能描述
///
/// 该宏从一个 JSON 配置文件中读取字符映射，并生成两个静态项：
///
/// - `ANSI_CODE_PAGE: u32`：源文本所使用的 ANSI 代码页
/// - `PHF_MAP: ::phf::Map<u16, u16>`：`u16 -> u16` 的字符映射表
///
/// 这里的映射表键值都直接使用 Unicode BMP 范围内的码点值；宏本身**不会**把字符预先转换成
/// Shift_JIS / GBK 等多字节编码后的数值表。代码页信息会单独保存在 `ANSI_CODE_PAGE` 中，供运行时按需使用。
///
/// # 输入参数
///
/// 接受一个字符串字面量参数：
///
/// - `mapping_path`：映射配置文件路径（相对于 `CARGO_MANIFEST_DIR`）
///
/// # 配置文件格式
///
/// 配置文件应为一个 JSON 对象，格式如下：
///
/// ```json
/// {
///   "code_page": 932,
///   "src_encoding": "ShiftJIS",
///   "mapping": {
///     "Ａ": "A",
///     "ｶ": "カ"
///   }
/// }
/// ```
///
/// 字段说明：
///
/// - `mapping`：必需。键和值都必须是单个 `char`，表示 1:1 字符映射。
/// - `code_page`：可选。直接指定源文本代码页，例如 `932`、`936`。
/// - `src_encoding`：可选。当前仅支持：
///   - `"ShiftJIS"` / `"CP932"` -> `932`
///   - `"GBK"` -> `936`
///
/// 若同时提供 `code_page` 与 `src_encoding`，优先使用 `code_page`。
/// 若两者都未提供，则 `ANSI_CODE_PAGE` 为 `0`。
///
/// # 输出
///
/// 宏展开后会生成：
///
/// ```rust
/// pub(super) static ANSI_CODE_PAGE: u32 = 932;
/// pub(super) static PHF_MAP: ::phf::Map<u16, u16> = ::phf::phf_map! {
///     0xFF21u16 => 0x0041u16,
///     0xFF76u16 => 0x30ABu16,
/// };
/// ```
///
/// 其中：
///
/// - `PHF_MAP` 的键：源字符的 Unicode `u16` 码点
/// - `PHF_MAP` 的值：目标字符的 Unicode `u16` 码点
///
/// # 示例
///
/// ```rust
/// generate_mapping_data!("assets/mapping.json");
/// ```
///
/// # 注意事项
///
/// - 路径相对于 `CARGO_MANIFEST_DIR`
/// - 所有字符必须位于 BMP 范围内（`<= 0xFFFF`），否则编译失败
/// - `mapping` 会按键排序后生成，以保证输出稳定
/// - `src_encoding` 目前只支持少量预设值，传入其他值会直接报编译错误
#[proc_macro]
pub fn generate_mapping_data(input: TokenStream) -> TokenStream {
    match impls::generate_mapping_data::generate_mapping_data(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 生成补丁数据的过程宏
///
/// # 功能描述
/// 这个宏通过比较原始文件和翻译文件，生成一个高效的补丁数据系统，用于在运行时动态替换文件内容。
/// 系统使用 SHA256 哈希值来标识原始文件，并通过 PHF（Perfect Hash Function）实现快速查找。
///
/// # 输入参数
/// 接受两个字符串字面量参数，用 `=>` 分隔：
/// - `raw_dir`: 原始文件目录的相对路径（相对于 `CARGO_MANIFEST_DIR`）
/// - `translated_dir`: 翻译文件目录的相对路径（相对于 `CARGO_MANIFEST_DIR`）
///
/// # 处理流程
/// 1. 扫描原始文件目录中的所有文件
/// 2. 在翻译文件目录中查找对应的翻译文件
/// 3. 验证原始文件和翻译文件的字节长度是否一致
/// 4. 计算原始文件的 SHA256 哈希值
/// 5. 生成压缩的静态数据和高效的查找结构
///
/// # 验证规则
/// - 原始文件和翻译文件必须存在且可读
/// - 原始文件和翻译文件的字节长度必须完全一致
/// - 原始文件的 SHA256 哈希值必须唯一（避免重复文件）
/// - 翻译文件目录中必须存在与原始文件同名的文件
///
/// # 性能特点
/// - 使用 PHF 实现 O(1) 时间复杂度的查找
/// - 翻译文件数据在编译时进行压缩
/// - 运行时按需解压缩（LazyLock延迟加载）
/// - 长度过滤器用于快速排除不匹配的文件
///
/// # 应用场景
/// 主要用于游戏修改、资源替换、本地化补丁等需要动态替换文件内容的场景，
/// 特别是在需要高效查找和最小化内存占用的环境中。
///
/// # 注意事项
/// - 文件按文件名进行匹配（翻译文件必须与原始文件同名）
/// - 所有文件都按二进制方式处理，不涉及字符编码转换
/// - 调试信息需要启用 `enable_debug_output` feature 才能使用
/// - 生成的静态变量都是 `pub(super)` 可见性
/// - 哈希比较使用字节数组，确保精确匹配
///
/// # 运行时使用示例
/// ```rust
/// // 1. 计算输入数据的长度和哈希
/// let input_len = input_data.len();
/// let input_hash = sha2::Sha256::digest(&input_data);
///
/// // 2. 使用长度过滤器快速排除
/// if LEN_FILTER.contains(&input_len) {
///     // 3. 在补丁映射中查找
///     if let Some(patched_data) = PATCHES.get(&input_hash) {
///         // 4. 使用找到的补丁数据
///         return patched_data.clone();
///     }
/// }
/// // 5. 返回原始数据（未找到补丁）
/// return input_data;
/// ```
#[proc_macro]
pub fn generate_patch_data(input: TokenStream) -> TokenStream {
    match impls::generate_patch_data::generate_patch_data(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 一个过程宏，用于根据指定目录下的 Rust 文件批量生成代码。
///
/// 这个宏会扫描指定目录下的所有 `.rs` 文件（除了 `mod.rs` 和 `lib.rs`），
/// 然后对每个文件应用提供的模板代码，生成相应的代码结构。
///
/// # 语法
/// ```ignore
/// expand_by_files!("relative/path/to/directory" => {
///     // 模板代码
///     // 可以使用以下占位符：
///     // - __file__: 文件名的下划线标识符 (如: my_module)
///     // - __file_str__: 文件名字符串字面量 (如: "my_module")
///     // - __file_pascal__: 文件名的大驼峰标识符 (如: MyModule)
/// }, { Optional, Exclude, Idents });
/// ```
///
/// # 文件过滤
/// - 只处理 `.rs` 扩展名的文件
/// - 自动跳过 `mod.rs` 和 `lib.rs` 文件
/// - 忽略子目录和非文件项
/// - 可选的排除列表：在模板后添加 `, { Ident1, Ident2 }` 格式，支持snake_case或PascalCase命名
///
/// # 示例
/// ```ignore
/// // 基础用法
/// expand_by_files!("src/models" => {
///     pub mod __file__;
///     pub use __file__::__file_pascal__;
/// });
///
/// // 带排除列表（忽略 code_cvt_hook.rs 和 file_hook.rs）
/// expand_by_files!("src/hook/traits" => {
///     #[cfg(feature = __file_str__)]
///     impl crate::hook::traits::__file_pascal__ for #name {}
/// }, {CodeCvtHook, FileHook});
/// ```
#[proc_macro]
pub fn expand_by_files(input: TokenStream) -> TokenStream {
    match impls::expand_by_files::expand_by_files(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 为 DLL 劫持生成导出转发包装器，并额外写出 `.def` 文件。
///
/// 该宏会在编译时分析指定目录中的唯一一个 DLL，读取其命名导出表，然后自动生成：
///
/// - 所有导出函数地址的静态变量
/// - 对应的裸函数跳转包装器（naked asm）
/// - `load_library()` / `unload_library()` 辅助函数
/// - 一个 `.def` 文件，用于保留原 DLL 的导出名与 ordinal
///
/// # 语法
///
/// ```ignore
/// generated_exports_from_hijacked_dll!("path/to/dll_dir" => "path/to/output.def");
/// ```
///
/// # 参数
///
/// - 左侧参数：包含目标 DLL 的目录路径（相对于 `CARGO_MANIFEST_DIR`）
/// - 右侧参数：生成的 `.def` 文件输出路径（相对于 `CARGO_MANIFEST_DIR`）
///
/// # 要求
///
/// - 指定目录必须存在
/// - 目录中必须且只能有一个 DLL 文件
/// - 该 DLL 必须包含**命名导出**；纯 ordinal 导出当前不支持
///
/// # 生成代码
///
/// 宏展开后会生成：
///
/// - `HMOD`：真实 DLL 的模块句柄
/// - `ADDR_*`：每个导出函数对应的目标地址缓存
/// - 若干 `#[unsafe(export_name = "...")]` 的跳转导出函数
/// - `pub(super) unsafe extern "system" fn load_library()`
/// - `pub(super) unsafe extern "system" fn unload_library()`
///
/// 其中 `load_library()` 会调用 crate 内的 Windows 辅助函数加载真实 DLL，并批量解析所有导出地址；
/// 每个导出包装器随后通过汇编直接跳转到解析出来的真实地址。
///
/// # 示例
///
/// ```ignore
/// generated_exports_from_hijacked_dll!(
///     "assets/hijacked" => "target/generated/version.def"
/// );
/// ```
///
/// # 注意事项
///
/// - 该宏专用于 Windows DLL 劫持/转发场景
/// - 生成代码依赖平台相关的 naked asm
/// - `.def` 文件会在宏展开期间直接写入磁盘
/// - 目前仅支持有名字的导出，不支持纯序号导出
#[proc_macro]
pub fn generated_exports_from_hijacked_dll(input: TokenStream) -> TokenStream {
    match impls::generate_exports_from_hijacked_dll::generated_exports_from_hijacked_dll(
        input.into(),
    ) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 生成文本补丁数据的编译时过程宏。
///
/// 该宏在编译时读取“原文目录”和“译文目录”中同名的 JSON 文件，生成数个 PHF 映射表，
/// 供运行时按“字典项优先、正文按上下文索引匹配”的方式快速查找译文。
///
/// # 语法
///
/// ```ignore
/// generated_text_patch_data! {
///     "path/to/raw_dir" => "path/to/translated_dir"
/// }
/// ```
///
/// # 输入要求
///
/// - 两个目录中的文件按**文件名**一一对应
/// - 每个文件内容都必须是 JSON 数组
/// - 原文数组与译文数组长度必须一致
/// - 每一项只会读取 `message`、`is_name`、`is_dict` 字段
///
/// 单项格式示例：
///
/// ```json
/// [
///   {"message": "原始消息"},
///   {"message": "另一条消息", "is_dict": true},
///   {"message": "角色名", "is_name": true}
/// ]
/// ```
///
/// # 生成内容
///
/// 宏展开后会生成以下项：
///
/// - `LookupResult`：查找结果结构，包含译文和命中的正文索引
/// - `DICT_PHF`：上下文无关字典映射，类型为 `phf::Map<&'static str, &'static str>`
/// - `TEXT_SINGLE_PHF`：无歧义正文映射，类型为 `phf::Map<&'static str, (usize, &'static str)>`
/// - `TEXT_MULTI_PHF`：多候选正文映射，类型为 `phf::Map<&'static str, &'static [(usize, &'static str)]>`
/// - `lookup_result(original, last_index)`：带上下文的统一查找接口
/// - `lookup(original)`：不带上下文的兼容接口
///
/// # 处理规则
///
/// - `is_name == true` 或 `is_dict == true` 的条目视为字典项，进入 `DICT_PHF`
/// - 其余非空 `message` 条目视为正文，并按遍历顺序分配稳定的 `text_index`
/// - 同一原文若只出现一次，进入 `TEXT_SINGLE_PHF`
/// - 同一原文若出现多次，则全部候选保留到 `TEXT_MULTI_PHF`
/// - 即使多个候选的译文完全相同，也**不会合并**，因为它们的上下文索引不同
/// - 字典项要求严格 1:1；若同一原文对应多个不同译文，会直接报编译错误
/// - 空 `message` 会被跳过
///
/// # 查找顺序
///
/// 运行时查找顺序为：
///
/// 1. `DICT_PHF`
/// 2. `TEXT_SINGLE_PHF`
/// 3. `TEXT_MULTI_PHF`（结合 `last_index` 选择最近候选）
///
/// 当命中 `TEXT_MULTI_PHF` 时：
///
/// - 若提供 `last_index`，会选择与之距离最近的正文项
/// - 若未提供 `last_index`，则默认选择第一个候选
///
/// # 示例
///
/// ```ignore
/// generated_text_patch_data! {
///     "texts/original" => "texts/chinese"
/// }
///
/// let translated = lookup("Hello world!").unwrap_or("Hello world!");
/// ```
#[proc_macro]
pub fn generated_text_patch_data(input: TokenStream) -> TokenStream {
    match impls::generate_text_patch_data::generate_text_patch_data(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 从1337补丁文件生成内存补丁函数的过程宏
///
/// 这个宏在编译时读取指定目录下的所有 `.1337` 补丁文件，生成一个函数，该函数在运行时将补丁数据写入对应模块的内存地址。
/// 主要用于游戏修改、热修复等需要动态修改二进制代码的场景。
///
/// # 语法
/// ```ignore
/// generate_patch_fn_from_1337! {
///     "path/to/patches" => <pub> fn function_name
/// }
/// ```
/// - 第一个参数：1337补丁文件所在目录（相对于 `CARGO_MANIFEST_DIR`）
/// - 可选的 `pub` 关键字：控制生成函数的可见性
/// - 第二个参数：生成的函数名
///
/// # 1337文件格式
/// 宏会读取目录下所有扩展名为 `.1337` 的文件，每个文件格式如下：
/// ```text
/// // 这是注释行
/// >game.exe              # 模块声明（以.exe结尾的为主模块）
/// 0x140001000: 0x74 -> 0xEB  # 补丁格式：地址: 原始字节 -> 新字节
/// 0x140001001: 0x3C -> 0x90
///
/// >engine.dll            # 另一个模块的声明
/// 0x180204A90: 0xE8 -> 0xC3
/// ```
///
/// ## 格式规则
/// - 模块声明：以 `>` 开头，后跟模块名（如 `game.exe` 或 `library.dll`）
/// - 补丁条目：`地址: 原始字节 -> 新字节`（地址为十六进制，字节值也必须为十六进制）
/// - 注释：以 `//` 或 `#` 开头的行会被忽略
/// - 空行：自动忽略
///
/// # 生成内容
/// 宏展开后会生成一个返回 `crate::Result<()>` 的函数，该函数：
/// - 自动获取各模块的基址（主模块使用空字符串获取）
/// - 按模块分组并应用所有补丁
/// - 自动合并连续地址的补丁以优化内存写入操作
///
/// # 处理规则
/// - 自动处理路径解析（相对于 `CARGO_MANIFEST_DIR`）
/// - **最多只能有一个主模块**（以 `.exe` 结尾的模块），否则编译失败
/// - 补丁必须位于模块声明之后，否则编译失败
/// - 自动按地址排序并合并连续的内存补丁
/// - 使用 `crate::utils::win32::get_module_handle` 获取模块句柄
/// - 使用 `crate::utils::mem::patch::write_asm` 写入补丁数据
///
/// # 示例
/// ## 1337文件内容 (`patches/game.1337`)
/// ```text
/// >MyGame.exe
/// 0x140001000: 0x74 -> 0xEB  # 将je指令改为jmp
/// 0x140001001: 0x3C -> 0x90  # 后续字节改为nop
/// 0x140001002: 0x07 -> 0x90
/// ```
///
/// ## 使用宏
/// ```ignore
/// generate_patch_fn_from_1337! {
///     "patches" => pub fn apply_game_patches
/// }
///
/// // 在main函数中调用
/// fn main() -> crate::Result<()> {
///     apply_game_patches()?;
///     Ok(())
/// }
/// ```
///
/// ## 生成的代码大致如下
/// ```ignore
/// pub fn apply_game_patches() -> crate::Result<()> {
///     // Patch模块: MyGame.exe
///     let module_base = crate::utils::win32::get_module_handle(core::ptr::null())? as usize;
///     let target_addr = module_base.wrapping_add(0x140001000 as usize);
///     let data: &[u8] = &[0xEB, 0x90, 0x90];
///     crate::utils::mem::patch::write_asm(target_addr as *mut u8, data)?;
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn generate_patch_fn_from_1337(input: TokenStream) -> TokenStream {
    match impls::generate_patch_fn_from_1337::generate_patch_fn_from_1337(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 为结构体自动生成默认的钩子实现的过程宏
///
/// 该 Derive 宏会扫描 `src/hook/traits` 目录下的所有 `.rs` 文件（除 `mod.rs` 和 `lib.rs` 外），
/// 解析其中真实声明的 trait，并为当前结构体逐个生成默认实现。
///
/// 与旧版本“按文件名推导 trait 名”不同，当前实现是**直接读取 trait 定义本身**，因此：
/// - 一个 traits 文件里即使包含多个 trait，也都会被正确处理；
/// - 生成的 impl 路径会指向真实模块路径，例如 `crate::hook::traits::filesystem::ReadFile`；
/// - 不再依赖“一个文件只对应一个 trait”的旧假设。
///
/// 此外，宏还会读取 `constant_assets/featured_hook_lists.json` 中的 `trait` 配置，
/// 对那些在特定 feature 组合下会由“专门实现”接管的 trait，自动为默认 impl 补上
/// `#[cfg(not(any(...)))]` 排除条件，避免默认实现与特化实现冲突。
///
/// # 生成的代码结构
/// ```ignore
/// impl crate::hook::traits::filesystem::CreateFile for MyTranslator {}
///
/// impl crate::hook::traits::filesystem::ReadFile for MyTranslator {}
///
/// // 若某 trait 被 featured 配置声明为特化 trait，则默认 impl 会被自动排除
/// #[cfg(not(any(...)))]
/// impl crate::hook::traits::gdi_text::TextHook for MyTranslator {}
/// ```
/// 上面只是示意，实际生成结果取决于 `src/hook/traits` 中声明的 trait，以及
/// `featured_hook_lists.json` 中的 `trait` 配置。
///
/// # 辅助属性
/// - `#[exclude(Ident1, Ident2, ...)]`：按 **trait 名** 排除指定默认实现，不为其生成 impl
/// - `Ident` 可以写成 snake_case 或 PascalCase，宏内部会统一转成 PascalCase 后再匹配 trait 名
///
/// # 示例
/// ```ignore
/// // 基础用法：为所有 trait 生成实现
/// #[derive(DefaultHook)]
/// struct MyTranslator;
///
/// // 排除特定 trait：不生成 MultiByteToWideChar 和 ReadFile 的默认实现
/// #[derive(DefaultHook)]
/// #[exclude(MultiByteToWideChar, ReadFile)]
/// struct MyTranslator;
/// ```
#[proc_macro_derive(DefaultHook, attributes(exclude))]
pub fn derive_default_hook(input: TokenStream) -> TokenStream {
    match impls::derive_default_hook::derive_default_hook(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 从 JSON 配置文件生成钩子的启用/禁用函数
///
/// 这个宏会在编译时读取两个 JSON 配置文件，根据条件配置和用户设置，
/// 自动生成运行时启用和禁用钩子的函数。支持条件编译和用户覆盖配置。
///
/// # 参数
/// - `featured_path`: 特性化钩子列表的 JSON 文件路径（相对于 `CARGO_MANIFEST_DIR`）
/// - `user_path`: 用户钩子列表的 JSON 文件路径（相对于 `CARGO_MANIFEST_DIR`）
///
/// 参数格式：`"featured.json", "user.json"`
///
/// # JSON 文件格式
///
/// ## 特性化钩子列表格式
/// ```json
/// {
///   "cfg_condition_1": {
///     "trait": ["SomeHookTrait"],
///     "fn": ["hook_name_1", "hook_name_2"]
///   },
///   "cfg_condition_2": {
///     "fn": ["hook_name_3"]
///   }
/// }
/// ```
/// 键为 Rust 的 `#[cfg(...)]` 条件，值为对象：
/// - `trait`：可选，声明在该条件下被视为“特化实现已接管”的 Hook trait 名列表
/// - `fn`：可选，声明在该条件下需要自动启用/禁用的 detour 函数名列表
///
/// 其中 `generate_hook_lists_from_json` 只使用 `fn` 字段；`trait` 字段则供
/// `#[derive(DefaultHook)]` 用于抑制默认 impl 生成。
///
/// ## 用户钩子列表格式
/// ```json
/// {
///   "enable": ["hook_name_a", "hook_name_b"],
///   "disable": ["hook_name_c"]
/// }
/// ```
/// - `enable`: 额外加入自动启用/自动禁用流程的钩子列表（覆盖特性化配置）
/// - `disable`: 从特性化自动列表中移除的钩子列表
///
/// # 生成代码
/// 宏展开后会生成以下两个函数：
/// - `enable_hooks_from_lists()` - 根据配置启用所有符合条件的钩子
/// - `disable_hooks_from_lists()` - 根据配置禁用所有已启用的钩子
///
/// 每个钩子通过 `generate_detour_ident` 生成对应的标识符，并调用其
/// `enable()` 或 `disable()` 方法。
///
/// # 配置解析规则
/// 1. 优先处理用户配置：`disable` 列表中的钩子会从任何条件中移除
/// 2. `enable` 列表中的钩子会额外加入自动启用/自动禁用流程
/// 3. `disable` 列表中的钩子只会从特性化自动列表里移除；它表示“不要由本宏自动启用”，并不意味着此处会主动调用对应 hook 的 `disable()`
/// 4. 用户配置中同一个钩子不能同时出现在 `enable` 和 `disable` 中
/// 5. 特性化配置中每个条件会生成对应的 `#[cfg(...)]` 代码块
/// 6. 空的条件配置（所有钩子都被用户配置覆盖）不会生成代码
///
/// # 示例
/// ```
/// generate_hook_lists_from_json!("hooks/featured.json", "hooks/user.json");
/// ```
///
/// 假设 `featured.json` 内容：
/// ```json
/// {
///   "target_os = \"windows\"": {
///     "fn": ["CreateWindowEx", "MessageBoxW"]
///   },
///   "all(feature = \"directx\", target_os = \"windows\")": {
///     "fn": ["Direct3DCreate9"]
///   }
/// }
/// ```
///
/// `user.json` 内容：
/// ```json
/// {
///   "enable": ["ExtraHook"],
///   "disable": ["MessageBoxW"]
/// }
/// ```
///
/// 生成的代码将根据编译条件启用相应的钩子，同时额外把 `ExtraHook`
/// 纳入自动启用/自动禁用流程，而 `MessageBoxW` 会从特性化自动启用列表中被排除。
#[proc_macro]
pub fn generate_hook_lists_from_json(input: TokenStream) -> TokenStream {
    match impls::generate_hook_lists_from_json::generate_hook_lists_from_json(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 生成资源包嵌入代码的过程宏。
///
/// 该宏在编译时将指定目录下的资源文件打包，并生成用于运行时提取和访问这些资源的代码。
///
/// # 语法
///
/// ```ignore
/// generate_resource_pack!(resource_dir, config_path);
/// generate_resource_pack!(resource_dir, config_path, output_path);
/// ```
///
/// # 参数
///
/// - `resource_dir`: 资源文件所在的目录路径（相对于 `Cargo.toml` 的字符串字面量）。
/// - `config_path`: JSON 配置文件路径（相对于 `Cargo.toml` 的字符串字面量），
///   文件中必须包含 `RESOURCE_PACK_NAME` 字段用于指定资源包名称。
/// - `output_path`（可选）: 若提供，资源包将输出为外部文件而非嵌入二进制；
///   运行时将从该路径加载 `.pak` 文件。
///
/// # 生成的模块内容
///
/// 宏会生成一个包含以下内容的内部模块：
///
/// - `get_temp_dir()`: 返回资源提取的临时目录路径（`&'static Path`）。
/// - `RESOURCE_PATHS`: 包含所有资源相对路径的 `phf::Set<&'static str>` 静态集合。
/// - `is_resource(path: &str) -> bool`: 检查给定路径是否为资源包中的文件。
/// - `extract() -> Result<()>`: 将资源提取到临时目录，自动处理压缩解压。
/// - `clean_up() -> Result<()>`: 清理临时目录。
#[proc_macro]
pub fn generate_resource_pack(input: TokenStream) -> TokenStream {
    match impls::generate_resource_pack::generate_resource_pack(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// 生成位图字体资源及访问代码的过程宏。
///
/// 该宏在编译时读取字体文件和配置，光栅化指定字符集，打包生成纹理图集（Atlas），
/// 并生成运行时访问字体元数据的静态代码。
///
/// # 语法
///
/// ```ignore
/// generate_bitmap_font!("path/to/font_config.json");
/// ```
///
/// # 配置文件格式
///
/// 参数指向一个 JSON 文件，包含以下字段：
///
/// - `font_path`: 字体文件路径（TTF/OTF，相对于 `Cargo.toml`）。
/// - `chars`: 需要生成的字符集合字符串。
/// - `font_size`: 字体大小（可选，默认 24）。
/// - `padding`: 图集中字符间距（可选，默认 2 像素）。
/// - `texture_max_width`: 纹理图集最大宽度（可选，默认 2048）。
///
/// 示例 `bitmap_font.json`：
///
/// ```json
/// {
///     "font_path": "assets/fonts/NotoSansSC-Regular.otf",
///     "chars": "abcdefghijklmnopqrstuvwxyz0123456789",
///     "font_size": 32,
///     "padding": 2,
///     "texture_max_width": 2048
/// }
/// ```
///
/// # 处理流程
///
/// 1. **去重**：对 `chars` 去重，保持首次出现顺序。
/// 2. **光栅化**：使用 `fontdue` 将每个字符渲染为灰度位图。
/// 3. **图集打包**：采用简单 Shelf 算法将字符打包到 2D 纹理，(0,0) 像素预留给 `WHITE_PIXEL`。
/// 4. **生成资源**：输出 `assets/temp/bitmap_font.bin` 二进制纹理文件。
/// 5. **代码生成**：生成包含字体元数据访问的 Rust 代码。
///
/// # 生成的内容
///
/// 宏会生成以下静态项（位于调用模块的 `super` 作用域）：
///
/// - `BITMAP_FONT`: 嵌入的二进制纹理数据（`&'static [u8]`）。
/// - `ATLAS_WIDTH` / `ATLAS_HEIGHT`: 纹理图集尺寸（`u32`）。
/// - `ASCENT`: 字体上沿高度（`i32`，用于基线计算）。
/// - `DESCENT`: 字体下沿高度（`i32`，负值）。
/// - `LINE_HEIGHT`: 行高（`usize`，`ASCENT - DESCENT`）。
/// - `WHITE_PIXEL`: 单像素白色字符信息，用于纯色矩形绘制。
/// - `CHAR_MAP`: `phf::Map<char, CharInfo>`，字符到纹理坐标的映射表。
///
/// # 限制
///
/// - 图集高度限制为 8192 像素，超出将编译错误。
/// - 仅支持单通道 R8 灰度纹理（每个像素 1 字节）。
/// - 字符位图宽度不能超过 `texture_max_width`。
#[proc_macro]
pub fn generate_bitmap_font(input: TokenStream) -> TokenStream {
    match impls::generate_bitmap_font::generate_bitmap_font(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
