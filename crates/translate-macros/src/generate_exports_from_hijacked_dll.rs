use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{LitStr, Result, parse::Parse, parse::ParseStream};

use anyhow::{Context, Result as AnyResult};
use convert_case::{Case, Casing};
use std::fs;
use std::path::PathBuf;

/// 简单解析输入格式：一个字符串字面量，表示相对于 CARGO_MANIFEST_DIR 的目录
struct PathsInput {
    path: LitStr,
}

impl Parse for PathsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        Ok(PathsInput { path })
    }
}

/// 对导出名做一个简单的 Rust 静态符号名转换：
/// - 非字母数字字符替换为下划线
/// - 转换为大写下划线风格： e.g. "GetFileVersionInfoA" -> "ADDR_GET_FILE_VERSION_INFO_A"
fn rust_static_name_from_export(export: &str) -> String {
    let mut s = export.to_case(Case::Snake).to_uppercase();
    s = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("ADDR_{}", s)
}

/// 从 bytes 里解析导出符号（只返回有名字的导出）
/// 使用 goblin::pe 解析 PE 导出表
fn parse_pe_exports(bytes: &[u8]) -> AnyResult<Vec<String>> {
    use goblin::Object;
    match Object::parse(bytes)? {
        Object::PE(pe) => {
            let mut names = Vec::new();
            for export in &pe.exports {
                if let Some(name) = export.name {
                    names.push(name.to_string());
                }
            }

            Ok(names)
        }
        other => Err(anyhow::anyhow!(
            "不是 PE 文件（解析结果：{:?}），无法从中提取导出",
            other
        )),
    }
}

/// 主过程宏实现
pub fn generated_exports_from_hijacked_dll(input: TokenStream) -> TokenStream {
    // 解析输入
    let parsed = syn::parse_macro_input!(input as PathsInput);

    // 在宏展开时获取 CARGO_MANIFEST_DIR
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("无法获取 CARGO_MANIFEST_DIR（请在 Cargo 环境中编译）");

    // 拼接路径（相对于 manifest dir）
    let mapping_path = PathBuf::from(&manifest_dir).join(parsed.path.value());

    // 开始在编译期读取目录并处理
    let generated = match try_generate(&mapping_path) {
        Ok(tokens) => tokens,
        Err(e) => {
            // 如果出错：在编译期 panic（以便用户在编译时看到错误并修正）
            return syn::Error::new(Span::call_site(), format!("{:#}", e))
                .to_compile_error()
                .into();
        }
    };

    generated.into()
}

/// 实际工作函数（返回 quote! tokens）
fn try_generate(path: &PathBuf) -> AnyResult<proc_macro2::TokenStream> {
    // 检查目录存在
    let metadata = fs::metadata(path).with_context(|| format!("路径不存在：{}", path.display()))?;
    if !metadata.is_dir() {
        anyhow::bail!(
            "{} 不是目录，请传入包含 DLL 的目录路径（相对于 CARGO_MANIFEST_DIR）",
            path.display()
        );
    }

    // 列出目录
    let mut dlls = Vec::new();
    for entry in fs::read_dir(path).with_context(|| format!("无法读取目录 {}", path.display()))?
    {
        let e = entry?;
        let ft = e.file_type()?;
        if ft.is_file() {
            dlls.push(e.path());
        }
    }

    if dlls.len() != 1 {
        anyhow::bail!(
            "目录 {} 应该只包含一个 DLL 文件，实际找到 {} 个",
            path.display(),
            dlls.len()
        );
    }

    let dll_path = &dlls[0];
    let dll_basename = dll_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap()
        .to_string();

    let bytes =
        fs::read(dll_path).with_context(|| format!("无法读取 DLL 文件：{}", dll_path.display()))?;

    let export_names = parse_pe_exports(&bytes)
        .with_context(|| format!("解析 DLL 导出表失败：{}", dll_path.display()))?;

    if export_names.is_empty() {
        anyhow::bail!(
            "在 {} 中未找到命名导出（no named exports）",
            dll_path.display()
        );
    }

    // 生成静态声明 tokens
    let mut statics = Vec::new();
    let mut asm_fns = Vec::new();
    let mut c_string_literals = Vec::new();
    let mut addr_idents = Vec::new();

    for name in &export_names {
        // 生成静态名
        let static_name = rust_static_name_from_export(name);
        let ident = format_ident!("{}", static_name);
        addr_idents.push(ident.clone());

        // 生成 C 风格字符串字面（带 NUL 结尾）
        let cname = format!("{}\0", name);
        c_string_literals.push(cname);

        // static mut ADDR_XXX: usize = 0;
        let st = quote! {
            // 存放导出函数地址（运行时由 load_library 填充）
            static mut #ident: usize = 0;
        };
        statics.push(st);

        // 生成 wrapper 函数，named export 保持原名（使用 export_name 属性）
        // 使用裸汇编 jmp [ADDR_VAR] 的模式
        let export_name = name.clone();
        let export_fn_ident = format_ident!("lib_{}", export_name); // 内部函数名（不导出）
        // 确保函数名是合法标识符：若原导出名包含不可作为 ident 的字符，此处生成的内部 ident 仅用于 Rust 层，导出名保留为 export_name
        // 裸汇编：使用 std::arch::naked_asm! 宏
        let asm = quote! {
            #[unsafe(naked)]
            #[unsafe(link_section = ".text")]
            #[unsafe(export_name = #export_name)]
            pub unsafe extern "system" fn #export_fn_ident() {
                ::std::arch::naked_asm!(
                    "jmp [{0}]",
                    sym #ident,
                );
            }
        };
        asm_fns.push(asm);
    }

    // HMOD static
    let hmod_static = quote! {
        // 保存我们加载（劫持）的模块句柄
        static mut HMOD: usize = 0;
    };

    // 生成 load_library 函数（中文注释）
    // 注意我们构造了一个 &[ *const i8 ] 的数组，但在生成代码时写成 C 风格字节字符串，然后在运行时把它们当作指针传递
    let c_literals_iter = c_string_literals.iter();
    let c_lits_tokens: Vec<proc_macro2::TokenStream> = c_literals_iter
        .map(|s| {
            // 生成一个静态字节串常量，例如 b"GetFileVersionInfoA\0"；
            // 在最终的 load_library 函数中我们会用 `.as_ptr()` 传给 hook_utils
            let lit = proc_macro2::Literal::byte_string(s.as_bytes());
            quote! { #lit }
        })
        .collect();

    // 将地址静态 ident 列表用于 load assignment
    let addr_assigns: Vec<proc_macro2::TokenStream> = addr_idents
        .iter()
        .enumerate()
        .map(|(i, ident)| {
            let idx = proc_macro2::Literal::usize_unsuffixed(i);
            quote! {
                #ident = addrs[#idx] as usize;
            }
        })
        .collect();

    // 构造 load_library 函数体
    let load_fn = quote! {
        #[allow(static_mut_refs)]
        pub(super) unsafe extern "system" fn load_library() {
            // 在运行时从 crate::hook_utils 加载被劫持的真实 DLL 并解析符号地址
            // 1) 使用 crate::hook_utils::load_hijacked_library 以确保加载目标真实模块（例如 version.dll）
            // 2) 使用 crate::hook_utils::get_module_symbol_addrs_from_handle 来一次性获取我们需要的导出地址数组
            // 3) 将地址写入上面生成的静态变量
            unsafe {
                // 加载真实 DLL（由调用者提供 hook_utils 的实现）
                let hmod = crate::hook_utils::load_hijacked_library(#dll_basename)
                    .expect("Could not find target DLL");
                // 准备导出名的 C 字节串数组
                let names: &[&[u8]] = &[
                    #(#c_lits_tokens),*
                ];
                // 将 names 转成指针数组（null-terminated C 字符串指针）
                let name_ptrs: Vec<*const i8> = names.iter().map(|s| s.as_ptr() as *const i8).collect();

                // 使用 hook_utils 提供的辅助函数批量获取地址
                let addrs = crate::hook_utils::get_module_symbol_addrs_from_handle(
                    hmod,
                    &name_ptrs
                ).expect("Could not get symbol addrs for target DLL");

                // 保存模块句柄
                HMOD = hmod as usize;

                // 将返回的地址写入每个静态变量
                #(#addr_assigns)*
            }
        }
    };

    // 构造 unload_library 函数
    let reset_addr_statements: Vec<proc_macro2::TokenStream> = addr_idents
        .iter()
        .map(|ident| {
            quote! { #ident = 0; }
        })
        .collect();

    let unload_fn = quote! {
        #[allow(static_mut_refs)]
        pub(super) unsafe extern "system" fn unload_library() {
            unsafe {
                ::winapi::um::libloaderapi::FreeLibrary(HMOD as _);

                HMOD = 0;
                #(#reset_addr_statements)*
            };
        }
    };

    // 组合全部生成项：HMOD、所有 statics、所有 asm wrapper、load/unload 函数
    let output = quote! {

        #hmod_static

        #(#statics)*

        #(#asm_fns)*

        #load_fn

        #unload_fn
    };

    Ok(output)
}
