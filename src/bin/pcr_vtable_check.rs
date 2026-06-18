/// brocolib 测试工具：解析 PCR global-metadata.dat，提取指定类的 vtable 信息
/// 用于验证 brocolib 能否离线解析 vtable slot → 方法名
///
/// 用法: pcr_vtable_check <global-metadata.dat> [class_name_filter]
use brocolib::global_metadata::{
    self, DecodedMethodIndex, GlobalMetadata, TypeDefinitionIndex,
};
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

    // 只用 global metadata 解析
    let gmd = global_metadata::deserialize(&meta_data).unwrap_or_else(|e| {
        eprintln!("解析 global-metadata.dat 失败: {}", e);
        process::exit(1);
    });

    println!("=== PCR global-metadata.dat 概览 ===");
    println!("类型定义数: {}", gmd.type_definitions.as_vec().len());
    println!("方法定义数: {}", gmd.methods.as_vec().len());
    println!("程序集数:   {}", gmd.assemblies.as_vec().len());
    println!("VTable 行数: {}", gmd.vtable_methods.as_vec().len());

    // 遍历所有 type definition，找匹配 filter 的类
    println!("\n=== 遍历类型定义 ===");
    let type_defs = gmd.type_definitions.as_vec();
    let methods = gmd.methods.as_vec();
    let strings = &gmd.string;
    let vtable_methods = &gmd.vtable_methods;

    let mut found = 0;
    for (i, td) in type_defs.iter().enumerate() {
        let name: &str = &strings[td.name_index];
        let namespace: &str = &strings[td.namespace_index];
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
            let range = td.vtable_start.make_range(td.vtable_count as _);
            let vtbl = &vtable_methods[range];
            println!("  vtable slots ({}):", vtbl.len());
            for (slot, emi) in vtbl.iter().enumerate() {
                let decoded = emi.decode();
                let method_name = match &decoded {
                    DecodedMethodIndex::MethodDef(mi) => {
                        let md = &methods[mi.index() as usize];
                        let mname: &str = &strings[md.name_index];
                        format!("{} (method_def={})", mname, mi.index())
                    }
                    DecodedMethodIndex::TypeInfo(ti) => {
                        format!("TypeInfo({})", ti)
                    }
                    DecodedMethodIndex::Il2CppType(ti) => {
                        format!("Il2CppType({})", ti)
                    }
                    DecodedMethodIndex::FieldInfo(fri) => {
                        format!("FieldInfo({})", fri.index())
                    }
                    DecodedMethodIndex::StringLiteral(sli) => {
                        format!("StringLiteral({})", sli.index())
                    }
                    DecodedMethodIndex::MethodRef(mr) => {
                        format!("MethodRef({})", mr)
                    }
                    DecodedMethodIndex::FieldRva(fri) => {
                        format!("FieldRva({})", fri.index())
                    }
                    DecodedMethodIndex::Invalid(inv) => {
                        format!("Invalid({:?})", inv)
                    }
                };
                println!(
                    "    slot {:>3} {}  (encoded=0x{:08x})",
                    slot, method_name, emi.0
                );
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
