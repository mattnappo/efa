use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Mul,
    Div,
    Sub,
    Mod,
    Shl,
    Shr,
    And,
    Or,
    Eq,
    // BitAnd,
    // BitOr,
    // BitXor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instr {
    LoadArg(usize),
    LoadLocal(usize),
    LoadLit(usize),
    StoreLocal(usize),
    Pop,
    Dup,

    LoadFunc(Hash),
    LoadDyn(String),
    Call,
    CallSelf,
    Return,
    ReturnVal,

    Jump(usize),
    JumpT(usize),
    JumpF(usize),
    JumpEq(usize),
    JumpNe(usize),
    JumpGt(usize),
    JumpGe(usize),
    JumpLt(usize),
    JumpLe(usize),

    BinOp(BinOp),
    UnaryOp(UnaryOp),

    LoadArray,
    StoreArray,
    MakeArray,
    MakeSlice,
    StoreSlice,

    LoadField,
    StoreField,
    MakeStruct,

    Dbg,
    Nop,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bytecode {
    code: Vec<Instr>,
}

impl Bytecode {
    pub fn new(code: Vec<Instr>) -> Bytecode {
        Bytecode { code }
    }

    pub fn format_with_labelnames(bytecode: &Bytecode) -> Vec<String> {
        bytecode
            .code
            .iter()
            .map(|instr| match instr {
                Instr::Jump(i) => format!("    jmp L{i}"),
                Instr::JumpT(i) => format!("    jmp_t L{i}"),
                Instr::JumpF(i) => format!("    jmp_f L{i}"),
                Instr::JumpEq(i) => format!("    jmp_eq L{i}"),
                Instr::JumpNe(i) => format!("    jmp_ne L{i}"),
                Instr::JumpGt(i) => format!("    jmp_gt L{i}"),
                Instr::JumpGe(i) => format!("    jmp_ge L{i}"),
                Instr::JumpLt(i) => format!("    jmp_lt L{i}"),
                Instr::JumpLe(i) => format!("    jmp_le L{i}"),
                _ => format!("    {instr}"),
            })
            .collect()
    }
}

impl Deref for Bytecode {
    type Target = Vec<Instr>;

    fn deref(&self) -> &Self::Target {
        &self.code
    }
}

#[macro_export]
macro_rules! bytecode {
    ($($instr:expr),*) => {
        $crate::bytecode::Bytecode::new(vec![$($instr),*])
    };
}

impl fmt::Display for Bytecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self
            .code
            .iter()
            .map(|i| format!("    {i}"))
            .collect::<Vec<_>>();
        write!(f, "{}", &code[..].join("\n"))
    }
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Instr::LoadArg(i) => format!("load_arg {i}"),
                Instr::LoadLocal(i) => format!("load_loc {i}"),
                Instr::LoadLit(i) => format!("load_lit {i}"),
                Instr::StoreLocal(i) => format!("store_loc {i}"),
                Instr::Pop => "pop".to_string(),
                Instr::Dup => "dup".to_string(),

                Instr::LoadFunc(h) => format!("load_func 0x{}", hex::encode(h)),
                Instr::LoadDyn(s) => format!("load_dyn {s}"),
                Instr::Call => "call".to_string(),
                Instr::CallSelf => "call_self".to_string(),
                Instr::Return => "ret".to_string(),
                Instr::ReturnVal => "ret_val".to_string(),

                Instr::Jump(i) => format!("jmp {i}"),
                Instr::JumpT(i) => format!("jmp_t {i}"),
                Instr::JumpF(i) => format!("jmp_f {i}"),
                Instr::JumpEq(i) => format!("jmp_eq {i}"),
                Instr::JumpNe(i) => format!("jmp_ne {i}"),
                Instr::JumpGt(i) => format!("jmp_gt {i}"),
                Instr::JumpGe(i) => format!("jmp_ge {i}"),
                Instr::JumpLt(i) => format!("jmp_lt {i}"),
                Instr::JumpLe(i) => format!("jmp_le {i}"),

                Instr::BinOp(op) => format!("{op}"),
                Instr::UnaryOp(op) => format!("{op}"),

                Instr::LoadArray => "load_arr".to_string(),
                Instr::StoreArray => "store_arr".to_string(),
                Instr::MakeArray => "make_arr".to_string(),
                Instr::MakeSlice => "make_slice".to_string(),
                Instr::StoreSlice => "store_slice".to_string(),

                Instr::LoadField => "load_field".to_string(),
                Instr::StoreField => "store_field".to_string(),
                Instr::MakeStruct => "make_struct".to_string(),

                Instr::Dbg => "dbg".to_string(),
                Instr::Nop => "nop".to_string(),
            }
        )
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinOp::Add => "add",
                BinOp::Mul => "mul",
                BinOp::Div => "div",
                BinOp::Sub => "sub",
                BinOp::Mod => "mod",
                BinOp::Shl => "shl",
                BinOp::Shr => "shr",
                BinOp::And => "and",
                BinOp::Or => "or",
                BinOp::Eq => "eq",
            }
        )
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnaryOp::Not => "not",
                UnaryOp::Neg => "neg",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytecode_macro() {
        bytecode![];
        bytecode![Instr::Nop];
        bytecode![Instr::Nop, Instr::BinOp(BinOp::Add)];
    }
}
