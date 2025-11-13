use anyhow::Context;
use convert_case::{Case, Casing};
use goblin::Object;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::path::PathBuf;
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::utils::get_full_path_by_manifest;

struct PathsInput {
    hijacked_dll_dir: LitStr,
    def_output_path: LitStr,
}

impl Parse for PathsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let hijacked_dll_dir: LitStr = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let def_output_path: LitStr = input.parse()?;
        Ok(PathsInput {
            hijacked_dll_dir,
            def_output_path,
        })
    }
}

pub fn generated_exports_from_hijacked_dll(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<PathsInput>(input)?;
    let hijacked_dll_dir = get_full_path_by_manifest(parsed.hijacked_dll_dir.value()).unwrap();
    let def_output_path = get_full_path_by_manifest(parsed.def_output_path.value()).unwrap();

    let generated = match try_generate(&hijacked_dll_dir, &def_output_path) {
        Ok(tokens) => tokens,
        Err(e) => {
            syn_bail!(parsed.hijacked_dll_dir, "{e}");
        }
    };

    Ok(generated)
}

fn try_generate(dll_dir: &PathBuf, def_output_path: &PathBuf) -> anyhow::Result<TokenStream> {
    // 检查目录存在
    let metadata =
        std::fs::metadata(dll_dir).with_context(|| format!("路径不存在：{}", dll_dir.display()))?;
    if !metadata.is_dir() {
        anyhow::bail!(
            "{} 不是目录，请传入包含 DLL 的目录路径（相对于 CARGO_MANIFEST_DIR）",
            dll_dir.display()
        );
    }

    // 列出目录（只取文件）
    let mut dlls = Vec::new();
    for entry in
        std::fs::read_dir(dll_dir).with_context(|| format!("无法读取目录 {}", dll_dir.display()))?
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
            dll_dir.display(),
            dlls.len()
        );
    }

    let dll_path = &dlls[0];
    let dll_basename = dll_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap()
        .to_string();

    let bytes = std::fs::read(dll_path)
        .with_context(|| format!("无法读取 DLL 文件：{}", dll_path.display()))?;

    // 获取 (name, ordinal)
    let export_pairs = parse_pe_exports(&bytes)
        .with_context(|| format!("解析 DLL 导出表失败：{}", dll_path.display()))?;

    if export_pairs.is_empty() {
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

    // 为 .def 输出准备内容（LIBRARY + EXPORTS）
    // LIBRARY 使用不带扩展名的文件名作为模块名
    let library_name = dll_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&dll_basename)
        .to_string();

    // 收集 .def 的每一行（EXPORTS 下的行），格式：Name @ordinal
    let mut def_export_lines: Vec<String> = Vec::new();

    for (name, ordinal) in export_pairs.iter() {
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
        let export_name = name.clone();
        let export_fn_ident = format_ident!("lib_{}", export_name); // 内部函数名（不导出）
        let asm = quote! {
            #[unsafe(naked)]
            #[unsafe(link_section = ".text")]
            #[unsafe(export_name = #export_name)]
            pub unsafe extern "system" fn #export_fn_ident() {
                ::core::arch::naked_asm!(
                    "jmp [{0}]",
                    sym #ident,
                );
            }
        };
        asm_fns.push(asm);

        // 准备 def 行；保持与原 DLL 的 ordinal 一致
        // def 格式：<Name> @<ordinal>
        def_export_lines.push(format!("    {name} @{ordinal}"));
    }

    // HMOD static
    let hmod_static = quote! {
        // 保存我们加载（劫持）的模块句柄
        static mut HMOD: usize = 0;
    };

    // 生成 load_library 函数
    let c_literals_iter = c_string_literals.iter();
    let c_lits_tokens: Vec<TokenStream> = c_literals_iter
        .map(|s| {
            let lit = Literal::byte_string(s.as_bytes());
            quote! { #lit.as_ptr() }
        })
        .collect();

    // 将地址静态 ident 列表用于 load assignment
    let addr_assigns: Vec<TokenStream> = addr_idents
        .iter()
        .enumerate()
        .map(|(i, ident)| {
            let idx = Literal::usize_unsuffixed(i);
            quote! {
                #ident = addrs[#idx] as usize;
            }
        })
        .collect();

    // 构造 load_library 函数体
    let load_fn = quote! {
        #[allow(static_mut_refs)]
        pub(super) unsafe extern "system" fn load_library() {
            // 在运行时从 crate::utils::win32 加载被劫持的真实 DLL 并解析符号地址
            // 1) 使用 crate::utils::win32::load_hijacked_library 以确保加载目标真实模块（例如 version.dll）
            // 2) 使用 crate::utils::win32::get_module_symbol_addrs_from_handle 来一次性获取我们需要的导出地址数组
            // 3) 将地址写入上面生成的静态变量
            unsafe {
                // 加载真实 DLL
                let hmod = crate::utils::win32::load_hijacked_library(#dll_basename)
                        .expect("Could not find target DLL");

                // 使用 crate 提供的辅助函数批量获取地址
                let addrs = crate::utils::win32::get_module_symbol_addrs_from_handle(
                    hmod,
                    &[
                        #(#c_lits_tokens),*
                    ]
                ).expect("Could not get symbol addrs for target DLL");

                // 保存模块句柄
                HMOD = hmod as usize;

                // 将返回的地址写入每个静态变量
                #(#addr_assigns)*
            }
        }
    };

    // 构造 unload_library 函数
    let reset_addr_statements: Vec<TokenStream> = addr_idents
        .iter()
        .map(|ident| {
            quote! { #ident = 0; }
        })
        .collect();

    let unload_fn = quote! {
        #[allow(static_mut_refs)]
        pub(super) unsafe extern "system" fn unload_library() {
            unsafe {
                ::windows_sys::Win32::Foundation::FreeLibrary(HMOD as _);

                HMOD = 0;
                #(#reset_addr_statements)*
            };
        }
    };

    // 组装 .def 内容
    let mut def_contents = String::new();
    def_contents.push_str(&format!("LIBRARY {library_name}\n\n"));
    def_contents.push_str("EXPORTS\n");
    for line in &def_export_lines {
        def_contents.push_str(line);
        def_contents.push('\n');
    }

    // 尝试写入文件（如果失败，返回错误）
    std::fs::create_dir_all(
        def_output_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("")),
    )
    .with_context(|| format!("无法创建 def 输出目录：{}", def_output_path.display()))?;

    std::fs::write(def_output_path, def_contents)
        .with_context(|| format!("无法写入 def 文件：{}", def_output_path.display()))?;

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
/// 返回 Vec<(name, ordinal)>，ordinal 为导出序号（基于 PE 的 ordinal base 计算的绝对序号）
/// TODO: 现在仍然不支持无名导出(即纯序号导出，需要的时候再实现吧)
fn parse_pe_exports(bytes: &[u8]) -> anyhow::Result<Vec<(String, u32)>> {
    let pe = match Object::parse(bytes)? {
        Object::PE(pe) => pe,
        other => {
            anyhow::bail!("不是 PE 文件（解析结果：{other:?}），无法从中提取导出");
        }
    };

    let export_data = pe
        .export_data
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("PE 文件没有导出表"))?;

    let ordinal_base = export_data.export_directory_table.ordinal_base;
    let ordinals = &export_data.export_ordinal_table;

    // 确保表长度一致（防止损坏的 PE）
    anyhow::ensure!(
        ordinals.len() == pe.exports.len(),
        "导出表损坏: ordinals 长度 {} != exports 长度 {}",
        ordinals.len(),
        pe.exports.len()
    );

    let names: Vec<_> = pe
        .exports
        .iter()
        .enumerate()
        .filter_map(|(i, export)| {
            let name = export.name?;
            let rel = ordinals.get(i).copied()?;
            let absolute = ordinal_base.saturating_add(rel as u32);
            Some((name.to_string(), absolute))
        })
        .collect();

    Ok(names)
}
