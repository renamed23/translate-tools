use std::path::PathBuf;

use anyhow::Context;
use goblin::Object;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use crate::utils::{
    find_single_file_in_dir, get_full_path_by_manifest, input::ArrowSeparatedPaths,
};

pub fn generated_exports_from_hijacked_dll(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<ArrowSeparatedPaths>(input)?;
    let hijacked_dll_dir = get_full_path_by_manifest(parsed.left.value())?;
    let def_output_path = get_full_path_by_manifest(parsed.right.value())?;

    let dll_path = find_single_file_in_dir(&hijacked_dll_dir, "dll", &parsed.left)?;

    let generated = match try_generate(&dll_path, &def_output_path) {
        Ok(tokens) => tokens,
        Err(e) => {
            syn_bail!(parsed.left, "{e}");
        }
    };

    Ok(generated)
}

fn try_generate(dll_path: &PathBuf, def_output_path: &PathBuf) -> anyhow::Result<TokenStream> {
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

    for (i, (name, ordinal)) in export_pairs.iter().enumerate() {
        // 1. 生成内部使用的标识符：如 ADDR_1, lib_export_1
        let static_ident = format_ident!("ADDR_{i}");
        let export_fn_ident = format_ident!("lib_export_{i}");

        addr_idents.push(static_ident.clone());

        // 2. 准备 C 风格字符串字面量（用于 GetProcAddress）
        let cname = format!("{}\0", name);
        c_string_literals.push(cname);

        // 3. 生成静态变量：用于存放函数地址
        statics.push(quote! {
            static #static_ident: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);
        });

        // 4. 生成 naked 跳转函数
        // 关键点：#[unsafe(export_name = #name)] 确保导出的符号与原 DLL 严格一致
        let export_name = name.clone();
        asm_fns.push(quote! {
            #[unsafe(naked)]
            #[unsafe(link_section = ".text")]
            #[unsafe(export_name = #export_name)]
            pub unsafe extern "system" fn #export_fn_ident() {
                #[cfg(target_arch = "x86_64")]
                ::core::arch::naked_asm!(
                    "jmp qword ptr [rip + {0}]",
                    sym #static_ident,
                );

                #[cfg(target_arch = "x86")]
                ::core::arch::naked_asm!(
                    "jmp dword ptr [{0}]",
                    sym #static_ident,
                );
            }
        });

        // 5. 准备 .def 行
        def_export_lines.push(format!("    {name} @{ordinal}"));
    }

    // HMOD static
    let hmod_static = quote! {
        // 保存我们加载（劫持）的模块句柄
        static HMOD: ::core::sync::atomic::AtomicPtr<::core::ffi::c_void> = ::core::sync::atomic::AtomicPtr::new(::core::ptr::null_mut());
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
                #ident.store(addrs[#idx], ::core::sync::atomic::Ordering::Release);
            }
        })
        .collect();

    // 构造 load_library 函数体
    let load_fn = quote! {
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
                    *hmod,
                    &[
                        #(#c_lits_tokens),*
                    ]
                ).expect("Could not get symbol addrs for target DLL");

                // 保存模块句柄
                HMOD.store(*hmod, ::core::sync::atomic::Ordering::Release);
                ::core::mem::forget(hmod);

                // 将返回的地址写入每个静态变量
                #(#addr_assigns)*
            }
        }
    };

    // 构造 unload_library 函数
    let reset_addr_statements: Vec<TokenStream> = addr_idents
        .iter()
        .map(|ident| {
            quote! { #ident.store(0, ::core::sync::atomic::Ordering::Release); }
        })
        .collect();

    let unload_fn = quote! {
        pub(super) unsafe extern "system" fn unload_library() {
            unsafe {
                let hmod = HMOD.swap(::core::ptr::null_mut(), ::core::sync::atomic::Ordering::AcqRel);
                ::windows_sys::Win32::Foundation::FreeLibrary(hmod);

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
