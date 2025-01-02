use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BinOp {
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
enum UnaryOp {
    Not = 0,
    Neg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Instr {
    LoadArg(u32),
    LoadLocal(u32),
    LoadLit(u32),
    StoreLocal(u32),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bytecode {
    code: Vec<Instr>,
}
