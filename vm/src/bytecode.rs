use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinOp {
    Add = 0,
    Mul,
    Div,
    Sub,
    Mod,
    Shl,
    Shr,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOp {
    Not = 0,
    Neg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instr {
    LoadArg(usize),
    LoadLocal(usize),
    LoadLit(usize),
    StoreLocal(usize),
    Pop,

    LoadFunc,
    Call,
    Return,
    ReturnValue,

    Jump,
    JumpEq,
    JumpGt,
    JumpGe,
    JumpLt,
    JumpLe,

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
