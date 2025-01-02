use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Shl, Shr, Sub};

use anyhow::{anyhow, bail, Result};

use super::bytecode::{BinOp, Bytecode, Instr, UnaryOp};

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
    Bool(bool),
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

    /// Start VM bytecode execution
    pub fn exec(&mut self) -> Result<()> {
        // let main_func = self.call_stack.last().unwrap();

        let bytecode = &self.call_stack.last().unwrap().code_obj.code;

        // main_func.instruction = 0;

        loop {
            if self.call_stack.last().unwrap().instruction >= bytecode.len() {
                break;
            }

            let instr = &bytecode[self.call_stack.last().unwrap().instruction];
            // self.exec_instr(instr);

            Vm::exec_instr(self, instr)?;

            // TODO: Will need to change when adding control flow
            self.call_stack.iter_mut().last().unwrap().instruction += 1;
        }

        Ok(())
    }

    /*
    pub fn exec2(&mut self) -> Result<()> {
        // Get the current frame from the call stack.
        let call_frame = self
            .call_stack
            .last_mut()
            .ok_or_else(|| anyhow!("no main func on stack"))?;

        let bytecode = &call_frame.code_obj.code;
        let bytecode_len = bytecode.len();

        while call_frame.instruction < bytecode_len {
            // Get the current instruction.
            let instruction = &bytecode[call_frame.instruction];

            // Execute the instruction.
            Vm::exec_instr(self, instruction)?;

            // Increment the instruction pointer for the next iteration.
            call_frame.instruction += 1;
        }

        Ok(())
    }
    */

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
            Instr::Pop => {
                stack.pop();
            }

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

            Instr::BinOp(op) => {
                if stack.len() < 2 {
                    bail!("cannot perform binary operation: stack underflow");
                }

                let arg1 = stack.pop().unwrap();
                let arg2 = stack.pop().unwrap();

                match op {
                    BinOp::Add => stack.push(arg1 + arg2),
                    BinOp::Mul => stack.push(arg1 * arg2),
                    BinOp::Div => stack.push(arg1 / arg2),
                    BinOp::Sub => stack.push(arg1 - arg2),
                    BinOp::Mod => stack.push(arg1 % arg2),
                    BinOp::Shl => stack.push(arg1 << arg2),
                    BinOp::Shr => stack.push(arg1 >> arg2),
                    BinOp::And => stack.push(arg1.and(arg2)),
                    BinOp::Or => stack.push(arg1.and(arg2)),
                }
            }
            Instr::UnaryOp(op) => {
                if stack.is_empty() {
                    bail!("cannot perform binary operation: stack underflow");
                }
                let arg = stack.pop().unwrap();

                match op {
                    UnaryOp::Not => stack.push(!arg),
                    UnaryOp::Neg => stack.push(-arg),
                }
            }

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

impl<'a> Add for Value<'a> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x + y),
            (Value::String(x), Value::String(y)) => Value::String(x + &y),
            _ => panic!("cannot add values of different type"),
        }
    }
}

impl<'a> Sub for Value<'a> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x - y),
            _ => panic!("failed to perform sub"),
        }
    }
}

impl<'a> Mul for Value<'a> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x * y),
            (Value::I32(x), Value::String(s)) | (Value::String(s), Value::I32(x)) => {
                Value::String(s.repeat(x as usize))
            }
            _ => panic!("failed to perform mul"),
        }
    }
}

impl<'a> Div for Value<'a> {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x / y),
            _ => panic!("failed to perform div"),
        }
    }
}

impl<'a> Rem for Value<'a> {
    type Output = Self;

    fn rem(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x % y),
            _ => panic!("failed to perform rem"),
        }
    }
}

impl<'a> Shl for Value<'a> {
    type Output = Self;

    fn shl(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x << y),
            _ => panic!("failed to perform shl"),
        }
    }
}

impl<'a> Shr for Value<'a> {
    type Output = Self;

    fn shr(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x >> y),
            _ => panic!("failed to perform shr"),
        }
    }
}

impl<'a> Neg for Value<'a> {
    type Output = Self;

    fn neg(self) -> Self {
        match self {
            Value::I32(x) => Value::I32(-x),
            _ => panic!("failed to perform neg"),
        }
    }
}

impl<'a> Not for Value<'a> {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Value::Bool(x) => Value::Bool(!x),
            _ => panic!("failed to perform not"),
        }
    }
}

impl<'a> Value<'a> {
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(((x != 0) && (y != 0)) as i32),
            (Value::String(s), Value::I32(x)) | (Value::I32(x), Value::String(s)) => {
                Value::I32((!s.is_empty() && (x != 0)) as i32)
            }
            _ => panic!("failed to perform shr"),
        }
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

    #[test]
    fn test_ops() {
        let obj = init_code_obj(Bytecode::new(vec![
            Instr::BinOp(BinOp::Add),
            Instr::BinOp(BinOp::Mul),
            Instr::BinOp(BinOp::Mod),
            Instr::BinOp(BinOp::Sub),
            Instr::UnaryOp(UnaryOp::Neg),
        ]));
        let mut vm = init_test_vm(&obj);

        vm.data_stack.push(Value::int(5));
        vm.data_stack.push(Value::int(4));
        vm.data_stack.push(Value::int(6));
        vm.data_stack.push(Value::int(3));
        vm.data_stack.push(Value::int(2));

        vm.exec().unwrap();

        assert_eq!(
            vm.data_stack.pop().unwrap(),
            Value::int(-((((2 + 3) * 6) % 4) - 5))
        );
    }
}
