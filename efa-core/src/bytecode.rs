use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instr {
    LoadArg(usize),
    LoadLocal(usize),
    LoadLit(usize),
    StoreLocal(usize),
    Pop,

    LoadFunc(Hash),
    Call,
    CallSelf,
    Return,

    Jump(usize),
    JumpT(usize),
    JumpF(usize),
    JumpEq(usize),
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
