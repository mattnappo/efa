use crate::vm::CodeObject;
use crate::vm::Value;
use crate::Hash;

pub fn disassemble_function(name: &str, _hash: &Hash, obj: &CodeObject) {
    println!("${name} {}:", obj.argcount);
    obj.litpool.iter().for_each(|lit| {
        println!(
            "    .lit {}",
            match lit {
                Value::I32(i) => format!("{i}"),
                Value::String(s) => format!("\"{s}\""),
                Value::Bool(b) => format!("{b}"),
                Value::Hash(h) => format!("0x{}", hex::encode(h)),
            }
        )
    });
    println!("{}", obj.code);
}
