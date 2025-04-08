use std::fmt::Write;

use crate::bytecode::Bytecode;
use crate::vm::CodeObject;
use crate::vm::Value;
use crate::Hash;

pub fn disassemble_function(
    name: &str,
    hash: &Hash,
    obj: &CodeObject,
) -> anyhow::Result<String> {
    let mut dis = String::new();

    // Function header
    writeln!(dis, "# 0x{}", hex::encode(hash))?;
    writeln!(dis, "${name} {}:", obj.argcount)?;

    // Literals
    obj.litpool.iter().try_for_each(|lit| {
        writeln!(
            dis,
            "    .lit {}",
            match lit {
                Value::String(s) => format!("\"{s}\""),
                Value::Hash(h) => format!("0x{}", hex::encode(h)),
                Value::I8(i) => format!("{i}"),
                Value::U8(u) => format!("{u}"),
                Value::I16(i) => format!("{i}"),
                Value::U16(u) => format!("{u}"),
                Value::I32(i) => format!("{i}"),
                Value::U32(u) => format!("{u}"),
                Value::I64(i) => format!("{i}"),
                Value::U64(u) => format!("{u}"),
                Value::I128(i) => format!("{i}"),
                Value::U128(u) => format!("{u}"),
                Value::Isize(i) => format!("{i}"),
                Value::Usize(u) => format!("{u}"),

                Value::F32(f) => format!("{f}"),
                Value::F64(f) => format!("{f}"),

                Value::Char(c) => format!("{c}"),
                Value::Bool(b) => format!("{b}"),
                Value::Container(_) => "<cont_obj>".to_string(), // TODO
            }
        )
    })?;

    // Rename labels in the jump instructions
    let mut code = Bytecode::format_with_labelnames(&obj.code);

    // Insert the labels into the bytecode
    obj.labels.iter().enumerate().fold(0, |k, (i, label)| {
        code.insert(label + k, format!("L{i}:"));
        k + 1
    });

    // Write out
    let code = code.as_slice().join("\n");
    writeln!(dis, "{}", code)?;
    Ok(dis)
}
