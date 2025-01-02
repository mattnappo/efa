use std::collections::HashMap;

use super::bytecode::{Bytecode, Instr};

const STACK_CAP: u32 = 256;

struct Vm<'a> {
    data_stack: Vec<Value<'a>>,
    call_stack: Vec<StackFrame<'a>>,
    data_stack_cap: u32,
    call_stack_cap: u32,
}

struct CodeObject<'a> {
    name: String,
    hash: &'a [u8; 32],
    litpool: Vec<Value<'a>>,
    argcount: u32,
    localnames: Vec<String>,

    code: Bytecode,
}

struct StackFrame<'a> {
    code_obj: &'a CodeObject<'a>,
    locals: HashMap<String, Value<'a>>,
    instruction: u32,
}

/// A value that can be on the stack.
enum Value<'a> {
    I32(i32),
    String(String),
    Hash(&'a [u8; 32]),
}

impl<'a> Vm<'a> {
    pub fn new() -> Vm<'a> {
        Vm {
            data_stack: Vec::new(),
            call_stack: Vec::new(),
            data_stack_cap: STACK_CAP,
            call_stack_cap: STACK_CAP,
        }
    }

    fn exec_instr(&mut self, instr: &Instr) {
        match instr {
            Instr::LoadArg(i) => {}
            Instr::LoadLocal(i) => {}
            Instr::LoadLit(i) => {}
            Instr::StoreLocal(i) => {}
            Instr::Pop => {}

            Instr::LoadFunc => {}
            Instr::Call => {}
            Instr::Return => {}
            Instr::ReturnValue => {}

            Instr::Jump => {}
            Instr::JumpEq => {}
            Instr::JumpGt => {}
            Instr::JumpGe => {}
            Instr::JumpLt => {}
            Instr::JumpLe => {}

            Instr::BinOp(op) => {}
            Instr::UnaryOp(op) => {}

            Instr::LoadArray => {}
            Instr::StoreArray => {}
            Instr::MakeArray => {}
            Instr::MakeSlice => {}
            Instr::StoreSlice => {}

            Instr::LoadField => {}
            Instr::StoreField => {}
            Instr::MakeStruct => {}

            Instr::Nop => {}
        }
    }
}
