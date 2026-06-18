/// brocolib 测试工具：解析 PCR global-metadata.dat，提取指定类的 vtable 信息
/// 用于验证 brocolib 能否离线解析 vtable slot → 方法名
///
/// 用法: pcr_vtable_check <global-metadata.dat> [class_name_filter]
use brocolib::global_metadata::{EncodedMethodIndex, GlobalMetadata};
use brocolib::Metadata;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: pcr_vtable_check <global-metadata.dat> [class_filter]");
        process::exit(1);
    }

    let meta_path = &args[1];
    let filter = args.get(2).map(|s| s.as_str());

    // 读 global-metadata.dat
    let meta_data = fs::read(meta_path).unwrap_or_else(|e| {
        eprintln!("无法读取 {}: {}", meta_path, e);
        process::exit(1);
    });

    // 只用 global metadata 解析（不需要 libil2cpp.so 的 ELF）
    // brocolib 的 Metadata::parse 需要 ELF，但 GlobalMetadata 可以单独 deserialize
    let gmd = brocolib::global_metadata::deserialize(&meta_data).unwrap_or_else(|e| {
        eprintln!("解析 global-metadata.dat 失败: {}", e);
        process::exit(1);
    });

    println!("=== PCR global-metadata.dat 概览 ===");
    println!("类型定义数: {}", gmd.type_definitions.as_vec().len());
    println!("方法定义数: {}", gmd.methods.as_vec().len());
    println!("字符串数:   {}", gmd.string.as_vec().len());
    println!("程序集数:   {}", gmd.assemblies.as_vec().len());
    println!("VTable 行数: {}", gmd.vtable_methods.as_vec().len());

    // 遍历所有 type definition，找匹配 filter 的类
    println!("\n=== 遍历类型定义 ===");
    let type_defs = gmd.type_definitions.as_vec();
    let strings = &gmd.string;
    let methods = &gmd.methods;
    let vtable_methods = &gmd.vtable_methods;

    let mut found = 0;
    for (i, td) in type_defs.iter().enumerate() {
        let name = td.name(&gmd);
        let namespace = td.namespace(&gmd);
        let full_name = if namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", namespace, name)
        };

        // 过滤
        if let Some(f) = filter {
            if !full_name.contains(f) {
                continue;
            }
        }

        found += 1;

        // 打印类基本信息
        println!(
            "\n--- {} (type_def_idx={}) ---",
            full_name, i
        );
        println!(
            "  字段: {}, 方法: {}, vtable: {}, 接口: {}",
            td.field_count, td.method_count, td.vtable_count, td.interfaces_count
        );

        // 读 vtable_methods
        if td.vtable_count > 0 {
            let vtbl = td.vtable_methods(&gmd);
            println!("  vtable slots ({}):", vtbl.len());
            for (slot, emi) in vtbl.iter().enumerate() {
                let decoded = emi.decode();
                let method_name = match &decoded {
                    brocolib::global_metadata::DecodedMethodIndex::MethodDef(mi) => {
                        let md = &methods[*mi];
                        format!("{} (method_def={})", md.name(&gmd), mi.index())
                    }
                    brocolib::global_metadata::DecodedMethodIndex::TypeInfo(ti) => {
                        format!("TypeInfo({})", ti)
                    }
                    brocolib::global_metadata::DecodedMethodIndex::Il2CppType(ti) => {
                        format!("Il2CppType({})", ti)
                    }
                    brocolib::global_metadata::DecodedMethodIndex::FieldInfo(fri) => {
                        format!("FieldInfo({})", fri.index())
                    }
                    brocolib::global_metadata::DecodedMethodIndex::StringLiteral(sli) => {
                        format!("StringLiteral({})", sli.index())
                    }
                    brocolib::global_metadata::DecodedMethodIndex::MethodRef(mr) => {
                        format!("MethodRef({})", mr)
                    }
                    brocolib::global_metadata::DecodedMethodIndex::FieldRva(fri) => {
                        format!("FieldRva({})", fri.index())
                    }
                    brocolib::global_metadata::DecodedMethodIndex::Invalid(inv) => {
                        format!("Invalid({:?})", inv)
                    }
                };
                println!("    slot {:3d}: {}  (encoded=0x{:08x})", slot, method_name, emi.0);
            }
        }

        if filter.is_some() && found >= 20 {
            println!("\n... 已显示 20 个类，停止（用更精确的 filter）");
            break;
        }
    }

    if filter.is_some() && found == 0 {
        println!("\n没有找到匹配 '{}' 的类", filter.unwrap());
    }

    println!("\n=== 完成 ===");
}
