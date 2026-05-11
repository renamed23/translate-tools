use std::{collections::HashMap, fs};

use goblin::Object;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use serde_json::Value;
use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::utils::{find_single_file_in_dir, get_full_path_by_manifest, read_optional_json_file};

struct Input {
    exe_dir: LitStr,
    config_path: LitStr,
    handler_fn: Ident,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut exe_dir = None;
        let mut config_path = None;
        let mut handler_fn = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;

            match key.to_string().as_str() {
                "exe_dir" => {
                    if exe_dir.is_some() {
                        syn_bail!(key, "重复的 `exe_dir`");
                    }
                    exe_dir = Some(input.parse::<LitStr>()?);
                }
                "config_path" => {
                    if config_path.is_some() {
                        syn_bail!(key, "重复的 `config_path`");
                    }
                    config_path = Some(input.parse::<LitStr>()?);
                }
                "handler_fn" => {
                    if handler_fn.is_some() {
                        syn_bail!(key, "重复的 `handler_fn`");
                    }
                    handler_fn = Some(input.parse::<Ident>()?);
                }
                other => syn_bail!(
                    key,
                    "未知参数 `{other}`, 预期 `exe_dir`, `config_path`, 或 `handler_fn`"
                ),
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let exe_dir = exe_dir.ok_or_else(|| input.error("未指定 `exe_dir` 参数"))?;
        let config_path = config_path.ok_or_else(|| input.error("未指定 `config_path` 参数"))?;
        let handler_fn = handler_fn.ok_or_else(|| input.error("未指定 `handler_fn` 参数"))?;

        Ok(Input {
            exe_dir,
            config_path,
            handler_fn,
        })
    }
}

pub fn generate_entry_point_hook(input: TokenStream) -> syn::Result<TokenStream> {
    let parsed = syn::parse2::<Input>(input)?;

    let exe_dir = &parsed.exe_dir;

    let exe_dir_full =
        get_full_path_by_manifest(exe_dir.value()).map_err(|e| syn_err!(exe_dir, "{e}"))?;

    let exe_path = find_single_file_in_dir(&exe_dir_full, "exe", exe_dir)?;
    let exe_display = exe_path.display().to_string();

    let exe_bytes = fs::read(&exe_path).map_err(|e| syn_err!(exe_dir, "读取exe失败: {e}"))?;

    let pe = match Object::parse(&exe_bytes)
        .map_err(|e| syn_err!(exe_dir, "解析PE失败 `{exe_display}`: {e}"))?
    {
        Object::PE(pe) => pe,
        other => syn_bail!(exe_dir, "不是PE文件: {other:?}"),
    };

    let machine = pe.header.coff_header.machine;
    match machine {
        goblin::pe::header::COFF_MACHINE_X86 => {}
        goblin::pe::header::COFF_MACHINE_X86_64 => {
            syn_bail!(exe_dir, "不支持 x64 PE，只支持 x86 (32-bit)");
        }
        m => syn_bail!(exe_dir, "不支持的CPU架构: 0x{m:04X}"),
    }

    let opt_header = pe
        .header
        .optional_header
        .ok_or_else(|| syn_err!(exe_dir, "PE没有OptionalHeader"))?;

    let image_base = opt_header.windows_fields.image_base;
    let entry_rva = opt_header.standard_fields.address_of_entry_point;
    let sections = &pe.sections;

    let config_path = get_full_path_by_manifest(parsed.config_path.value())
        .map_err(|e| syn_err!(&parsed.config_path, "{e}"))?;
    let config: HashMap<String, Value> = read_optional_json_file(&config_path)?;

    let hook_rva = if let Some(rva) = config.get("ENTRY_POINT_RVA").and_then(|v| v.as_u64()) {
        rva as u32
    } else {
        entry_rva
    };

    let hook_rva = resolve_short_jumps(&exe_bytes, sections, hook_rva, exe_dir)?;

    let file_offset =
        rva_to_file_offset(sections, hook_rva).map_err(|e| syn_err!(exe_dir, "{e}"))?;

    let ip = image_base + hook_rva as u64;
    let max_read = exe_bytes.len().saturating_sub(file_offset).min(128);
    let hook_bytes = &exe_bytes[file_offset..file_offset + max_read];

    let mut decoder =
        iced_x86::Decoder::with_ip(32, hook_bytes, ip, iced_x86::DecoderOptions::NONE);

    let mut total_len = 0usize;

    while total_len < 5 {
        let instr = decoder.decode();
        if instr.is_invalid() {
            syn_bail!(
                exe_dir,
                "在偏移 0x{hook_rva:X}+{total_len} 处解码到非法指令"
            );
        }

        let len = instr.len();

        if !is_safe_to_rebase(&instr) {
            syn_bail!(
                exe_dir,
                "在偏移 0x{hook_rva:X}+{total_len} 处发现不可移动的指令: {instr} (类型: {:?})",
                instr.flow_control()
            );
        }

        total_len += len;
    }

    let handler_name = parsed.handler_fn.to_string();
    let handler_fn = &parsed.handler_fn;
    let trampoline_ident = format_ident!("{handler_name}_trampoline");
    let init_ident = format_ident!("{handler_name}_init");
    let return_static_ident = format_ident!("{handler_name}_RETURN");

    let nop_byte_str = std::iter::repeat_n("0x90", total_len)
        .collect::<Vec<_>>()
        .join(", ");
    let asm_template = format!(
        "pushad\npushfd\ncall {{0}}\npopfd\npopad\n.byte {nop_byte_str}\njmp dword ptr [{{1}}]"
    );

    let hook_rva_lit = syn::LitInt::new(&format!("0x{hook_rva:X}"), Span::call_site());
    let total_len_lit = syn::LitInt::new(&format!("{total_len}"), Span::call_site());
    let trampoline_offset_lit = syn::LitInt::new("9", Span::call_site());

    Ok(quote! {
        static mut #return_static_ident: usize = 0;

        #[unsafe(naked)]
        #[unsafe(link_section = ".text")]
        unsafe extern "system" fn #trampoline_ident() {
            core::arch::naked_asm!(
                #asm_template,
                sym #handler_fn,
                sym #return_static_ident,
            )
        }

        pub fn #init_ident() -> crate::Result<()> {
            use crate::utils::exts::ptr_ext::{PtrWriteExt, PtrReadExt};
            let handle = crate::utils::win32::get_module_handle(core::ptr::null())?;
            let module = handle.cast::<u8>();

            unsafe {
                let hook_addr = module.add(#hook_rva_lit);

                #[cfg(feature = "enable_debug_output")]
                if hook_addr.read_unaligned() == 0xCC {
                    debug!("Warning: detect `INT3` at entry point");
                }

                let mut relocated = [0u8; #total_len_lit];
                hook_addr.copy_bytes_to(&mut relocated)?;

                let trampoline_ptr = #trampoline_ident as *mut u8;
                trampoline_ptr.add(#trampoline_offset_lit).patch_asm(&relocated)?;

                #return_static_ident = hook_addr.add(#total_len_lit) as usize;
                hook_addr.write_jmp_instruction(#trampoline_ident as _)?;
            }
            Ok(())
        }
    })
}

fn resolve_short_jumps(
    exe_bytes: &[u8],
    sections: &[goblin::pe::section_table::SectionTable],
    entry_rva: u32,
    lit: &LitStr,
) -> syn::Result<u32> {
    let mut rva = entry_rva;
    let max_follow = 8;
    for _ in 0..max_follow {
        let file_offset = rva_to_file_offset(sections, rva)
            .map_err(|_| syn_err!(lit, "短跳解析 @ 0x{rva:X}: RVA不在任何section"))?;
        if file_offset >= exe_bytes.len() {
            syn_bail!(lit, "短跳解析时文件偏移 0x{file_offset:X} 越界");
        }
        match exe_bytes[file_offset] {
            0xEB => {
                let rel = exe_bytes[file_offset + 1] as i8;
                let next = rva.wrapping_add(2);
                rva = (next as i64 + rel as i64) as u32;
            }
            _ => return Ok(rva),
        }
    }
    syn_bail!(lit, "短跳链太长 (>{max_follow})");
}

fn rva_to_file_offset(
    sections: &[goblin::pe::section_table::SectionTable],
    rva: u32,
) -> syn::Result<usize> {
    for sec in sections {
        let sec_start = sec.virtual_address;
        let sec_end = sec_start.wrapping_add(sec.virtual_size);
        if rva >= sec_start && rva < sec_end {
            let offset_in_sec = rva.wrapping_sub(sec_start) as usize;
            if offset_in_sec >= sec.size_of_raw_data as usize {
                syn_bail2!(
                    "RVA 0x{rva:X} 超出section `{}` 的原始数据范围",
                    sec_name(sec)
                );
            }
            return Ok(sec.pointer_to_raw_data as usize + offset_in_sec);
        }
    }
    syn_bail2!("RVA 0x{rva:X} 不属于任何section");
}

fn is_safe_to_rebase(instr: &iced_x86::Instruction) -> bool {
    use iced_x86::FlowControl;

    if instr.flow_control() != FlowControl::Next {
        return false;
    }

    if instr.is_ip_rel_memory_operand() {
        return false;
    }

    true
}

fn sec_name(sec: &goblin::pe::section_table::SectionTable) -> &str {
    std::str::from_utf8(&sec.name)
        .unwrap_or("?")
        .trim_end_matches('\0')
}
