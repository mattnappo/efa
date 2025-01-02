use std::collections::HashMap;

use anyhow::{bail, Result};

use super::bytecode::{Bytecode, Instr};

const STACK_CAP: usize = 256;

#[derive(Debug)]
struct Vm<'a> {
    data_stack: Vec<Value<'a>>,
    call_stack: Vec<StackFrame<'a>>,
    data_stack_cap: usize,
    call_stack_cap: usize,
}

#[derive(Debug, Clone)]
struct CodeObject<'a> {
    name: String,
    hash: [u8; 32],
    litpool: Vec<Value<'a>>,
    argcount: usize,
    localnames: Vec<String>,

    code: Bytecode,
}

#[derive(Debug, Clone)]
struct StackFrame<'a> {
    code_obj: &'a CodeObject<'a>,
    // TODO: Will need to make this a stack to keep track of nesting scope.
    locals: HashMap<String, Value<'a>>,
    instruction: usize,
}

/// A value that can be on the stack.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value<'a> {
    I32(i32),
    String(String),
    Hash(&'a [u8; 32]),
}

impl<'a> Value<'a> {
    pub fn int(i: i32) -> Value<'a> {
        Value::I32(i)
    }

    pub fn string(s: &str) -> Value<'a> {
        Value::String(s.to_string())
    }
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

    fn exec_instr(&mut self, instr: &Instr) -> Result<()> {
        let frame = self.call_stack.iter_mut().last().unwrap();
        let stack = &mut self.data_stack;

        match instr {
            Instr::LoadArg(i) => {
                if *i >= frame.code_obj.argcount {
                    bail!("argument index {i} out of bounds");
                }
                let arg_name = &frame.code_obj.localnames[*i];
                stack.push(frame.locals[arg_name].clone());
            }
            Instr::LoadLocal(i) => {
                let k = i + frame.code_obj.argcount;
                if k >= frame.code_obj.localnames.len() {
                    bail!("local index {k} out of bounds");
                }
                let arg_name = &frame.code_obj.localnames[k];
                stack.push(frame.locals[arg_name].clone());
            }
            Instr::LoadLit(i) => {
                stack.push(frame.code_obj.litpool[*i].clone());
            }
            Instr::StoreLocal(i) => {
                let k = i + frame.code_obj.argcount;
                let arg_name = &frame.code_obj.localnames[k];
                frame.locals.insert(arg_name.clone(), stack.pop().unwrap());
            }
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

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn init_code_obj<'a>(code: Bytecode) -> CodeObject<'a> {
        CodeObject {
            name: "testobj".to_string(),
            hash: [0; 32],
            litpool: vec![Value::int(5), Value::string("hello")],
            argcount: 2, // x and y
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_test_vm<'a>(code_obj: &'a CodeObject) -> Vm<'a> {
        let mut vm = Vm::new();

        let frame = StackFrame {
            code_obj,
            locals: HashMap::from([
                ("x".into(), Value::int(10)),
                ("y".into(), Value::string("ok")),
                ("z".into(), Value::int(64)),
            ]),
            instruction: 0,
        };

        vm.call_stack.push(frame);

        vm
    }

    #[test]
    fn test_load_arg() {
        let obj = init_code_obj(Bytecode::default());
        let mut vm = init_test_vm(&obj);

        vm.exec_instr(&Instr::LoadArg(0)).unwrap();
        let tos = vm.data_stack.pop().unwrap();
        assert!(matches!(tos, Value::I32(10)));

        vm.exec_instr(&Instr::LoadArg(1)).unwrap();
        let tos = vm.data_stack.pop().unwrap();
        assert!(matches!(tos, Value::String(ref s) if s == "ok"));
    }

    #[test]
    fn test_load_local() {
        let obj = init_code_obj(Bytecode::default());
        let mut vm = init_test_vm(&obj);

        vm.exec_instr(&Instr::LoadLocal(0)).unwrap();
        let tos = vm.data_stack.pop().unwrap();
        assert!(matches!(tos, Value::I32(64)));
    }

    #[test]
    fn test_load_lit() {
        let obj = init_code_obj(Bytecode::default());
        let mut vm = init_test_vm(&obj);

        vm.exec_instr(&Instr::LoadLit(1)).unwrap();
        let tos = vm.data_stack.pop().unwrap();
        assert!(matches!(tos, Value::String(ref s) if s == "hello"));
    }

    #[test]
    fn test_store_local() {
        let obj = init_code_obj(Bytecode::default());
        let mut vm = init_test_vm(&obj);

        let v = Value::I32(100);
        vm.data_stack.push(v.clone());
        vm.exec_instr(&Instr::StoreLocal(0)).unwrap();

        // Check
        let frame = vm.call_stack.iter().last().unwrap();
        assert_eq!(frame.locals.get("z".into()).unwrap().to_owned(), v);
    }
}
