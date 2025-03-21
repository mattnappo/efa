use std::fmt::{self, Write};

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
                Value::I32(i) => format!("{i}"),
                Value::String(s) => format!("\"{s}\""),
                Value::Bool(b) => format!("{b}"),
                Value::Hash(h) => format!("0x{}", hex::encode(h)),
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
