pub(crate) mod byte_slice;
pub(crate) mod expand_by_files;
pub(crate) mod ffi_catch_unwind;
pub(crate) mod flate;
pub(crate) mod generate_constants_from_json;
pub(crate) mod generate_detours;
pub(crate) mod generate_mapping_data;
pub(crate) mod generate_patch_data;
pub(crate) mod search_hook_impls;
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
    search_hook_impls::search_hook_impls(input)
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
    generate_constants_from_json::generate_constants_from_json(input)
}

/// 生成字符映射数据的过程宏
///
/// # 功能描述
/// 这个宏从映射配置文件和可选的译文文件中生成一个高效的字符映射表，用于在 Shift_JIS 编码和 UTF-16 编码之间进行转换。
/// 生成的映射表使用 Perfect Hash Function (PHF) 实现，提供 O(1) 时间复杂度的查找性能。
///
/// # 输入参数
/// 接受一个或两个字符串字面量参数，用逗号分隔：
/// - `mapping_path`: 必需，映射配置文件的相对路径（相对于 `CARGO_MANIFEST_DIR`）
/// - `translated_path`: 可选，译文文件的相对路径（用于生成完整映射数据）
///
/// # 配置文件格式
///
/// ## 映射配置文件 (mapping.json)
/// JSON 对象，键值对表示字符映射关系：
/// ```json
/// {
///   "原字符": "目标字符",
///   "Ａ": "A",
///   "ｶ": "カ"
/// }
/// ```
/// - 键和值都必须是单个 Unicode 字符
/// - 键字符必须属于 JIS0208 字符集（可被 Shift_JIS 编码）
///
/// ## 译文文件
/// 包含需要处理的文本内容，用于提取所有 JIS0208 字符并创建自映射（字符映射到自身）。
///
/// # 生成规则
/// 1. 如果提供了译文文件，首先提取其中所有 JIS0208 字符并创建自映射
/// 2. 使用映射配置文件中的映射关系覆盖或添加映射
/// 3. 将每个键字符编码为 Shift_JIS 双字节编码
/// 4. 将每个值字符转换为 UTF-16 码点
/// 5. 生成基于 PHF 的静态映射表
///
/// # 输出
/// 生成一个静态的 PHF 映射表：
/// ```rust
/// pub(super) static SJIS_PHF_MAP: ::phf::Map<u16, u16> = phf_map! {
///     0x8340u16 => 0x0041u16,  // "Ａ" -> "A"
///     0x8341u16 => 0x0042u16,  // "Ｂ" -> "B"
///     // ...
/// };
/// ```
///
/// # 使用示例
/// ## 基本用法（仅映射文件）
/// ```rust
/// generate_mapping_data!("assets/mapping.json");
/// ```
///
/// ## 完整映射（包含译文文件）
/// ```rust
/// generate_mapping_data!("assets/mapping.json", "assets/translated.txt");
/// ```
///
/// # 字符编码说明
/// - **键**: 原字符 -> Shift_JIS 双字节编码 -> u16（高字节在前）
/// - **值**: 目标字符 -> UTF-16 码点 -> u16
///
/// # 性能特点
/// - 使用 PHF 实现，编译时构建完美哈希函数
/// - 运行时查找时间复杂度 O(1)
/// - 适合在性能敏感的字符转换场景中使用
///
/// # 应用场景
/// 主要用于游戏本地化、字符集转换、文本处理等需要高效字符映射的场景，
/// 特别是在处理日文 Shift_JIS 编码文本时。
///
/// # 注意事项
/// - 配置文件路径相对于 `CARGO_MANIFEST_DIR`（项目根目录）
/// - 译文文件为可选，用于生成完整字符集的自映射
/// - 所有字符映射都是 1:1 的单个字符映射
/// - 目前不支持 BMP 之外的 Unicode 字符（>0xFFFF）
#[proc_macro]
pub fn generate_mapping_data(input: TokenStream) -> TokenStream {
    generate_mapping_data::generate_mapping_data(input)
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
/// - 翻译文件数据在编译时进行压缩（flate压缩）
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
/// - 调试信息需要启用 `debug_output` feature 才能使用
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
    generate_patch_data::generate_patch_data(input)
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
/// });
/// ```
///
/// # 文件过滤
/// - 只处理 `.rs` 扩展名的文件
/// - 自动跳过 `mod.rs` 和 `lib.rs` 文件
/// - 忽略子目录和非文件项
///
/// # 示例
/// ```ignore
/// expand_by_files!("src/models" => {
///     pub mod __file__;
///     pub use __file__::__file_pascal__;
/// });
/// ```
#[proc_macro]
pub fn expand_by_files(input: TokenStream) -> TokenStream {
    expand_by_files::expand_by_files(input)
}
