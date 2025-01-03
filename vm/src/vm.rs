use std::cmp::{Ord, Ordering};
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
    is_void: bool,
    localnames: Vec<String>,
    /// Map from label index to an offset in the bytecode
    labels: HashMap<usize, usize>,

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

impl<'a> PartialOrd for Value<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Some(x.cmp(&y)),
            _ => panic!("cannot compare non-integer values"),
        }
    }
}

impl<'a> Ord for Value<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => x.cmp(&y),
            _ => panic!("cannot compare non-integer values"),
        }
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

    /// Execute the frame at the top of the call stack.
    pub fn exec_top_frame(&mut self) -> Result<()> {
        let bytecode = &self.call_stack.last().unwrap().code_obj.code;
        while self.call_stack.last().unwrap().instruction < bytecode.len() {
            let instr = &bytecode[self.call_stack.last().unwrap().instruction];
            self.exec_instr(instr)?;
        }

        Ok(())
    }

    // TODO: pub fn exec_codeobj / exec_main
    // Wraps a code object in a frame and executes the frame
    // Need to deal with `locals` field of uninitialized local vars.

    fn exec_instr(&mut self, instr: &Instr) -> Result<()> {
        let frame = self.call_stack.iter_mut().last().unwrap();
        let stack = &mut self.data_stack;

        let mut next_instr = frame.instruction + 1;
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

            Instr::Jump(label) => next_instr = frame.code_obj.labels[label],
            Instr::JumpEq(label) => {
                if stack.len() < 2 {
                    bail!("cannot perform comparison: stack underflow");
                }

                let rhs = stack.pop().unwrap();
                let lhs = stack.pop().unwrap();

                if lhs == rhs {
                    next_instr = frame.code_obj.labels[label];
                }
            }
            Instr::JumpGt(label) => {
                if stack.len() < 2 {
                    bail!("cannot perform comparison: stack underflow");
                }

                let rhs = stack.pop().unwrap();
                let lhs = stack.pop().unwrap();

                if lhs > rhs {
                    next_instr = frame.code_obj.labels[label];
                }
            }
            Instr::JumpGe(label) => {
                if stack.len() < 2 {
                    bail!("cannot perform comparison: stack underflow");
                }

                let rhs = stack.pop().unwrap();
                let lhs = stack.pop().unwrap();

                if lhs >= rhs {
                    next_instr = frame.code_obj.labels[label];
                }
            }
            Instr::JumpLt(label) => {
                if stack.len() < 2 {
                    bail!("cannot perform comparison: stack underflow");
                }

                let rhs = stack.pop().unwrap();
                let lhs = stack.pop().unwrap();

                if lhs < rhs {
                    next_instr = frame.code_obj.labels[label];
                }
            }
            Instr::JumpLe(label) => {
                if stack.len() < 2 {
                    bail!("cannot perform comparison: stack underflow");
                }

                let rhs = stack.pop().unwrap();
                let lhs = stack.pop().unwrap();

                if lhs <= rhs {
                    next_instr = frame.code_obj.labels[label];
                }
            }

            Instr::BinOp(op) => {
                if stack.len() < 2 {
                    bail!("cannot perform binary operation: stack underflow");
                }

                let rhs = stack.pop().unwrap();
                let lhs = stack.pop().unwrap();

                match op {
                    BinOp::Add => stack.push(lhs + rhs),
                    BinOp::Mul => stack.push(lhs * rhs),
                    BinOp::Div => stack.push(lhs / rhs),
                    BinOp::Sub => stack.push(lhs - rhs),
                    BinOp::Mod => stack.push(lhs % rhs),
                    BinOp::Shl => stack.push(lhs << rhs),
                    BinOp::Shr => stack.push(lhs >> rhs),
                    BinOp::And => stack.push(lhs.and(rhs)),
                    BinOp::Or => stack.push(lhs.and(rhs)),
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

        frame.instruction = next_instr;

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
            is_void: false,
            labels: HashMap::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_code_obj_with_pool<'a>(code: Bytecode, litpool: Vec<Value<'a>>) -> CodeObject<'a> {
        CodeObject {
            name: "testobj".to_string(),
            hash: [0; 32],
            litpool,
            argcount: 2, // x and y
            is_void: false,
            labels: HashMap::new(),
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

        vm.exec_top_frame().unwrap();

        // 3 + 2
        // 6 * 5
        // 4 % 30
        // 5 - 4
        // -1
        assert_eq!(
            vm.data_stack.pop().unwrap(),
            Value::int(-(5 - (4 % (6 * (3 + 2)))))
        );
    }

    #[test]
    fn test_jump() {
        let mut obj = init_code_obj(Bytecode::new(vec![
            Instr::Jump(0),
            Instr::BinOp(BinOp::Add),
            Instr::BinOp(BinOp::Mul),
        ]));
        obj.labels.insert(0, 2);
        let mut vm = init_test_vm(&obj);

        vm.data_stack.push(Value::int(5));
        vm.data_stack.push(Value::int(4));

        vm.exec_top_frame().unwrap();

        assert_eq!(vm.data_stack.pop().unwrap(), Value::int(20));
    }

    #[test]
    fn test_jump_greater() {
        let mut obj = init_code_obj_with_pool(
            Bytecode::new(vec![
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(0), // 1
                Instr::LoadLit(1), // 2
                Instr::JumpGt(0),  // False (since 1 > 2 is false)
                Instr::BinOp(BinOp::Add),
                Instr::BinOp(BinOp::Mul),
            ]),
            vec![Value::int(1), Value::int(2), Value::int(4)],
        );
        obj.labels.insert(0, 7);
        let mut vm = init_test_vm(&obj);

        vm.data_stack.push(Value::int(1));
        vm.data_stack.push(Value::int(2));
        vm.data_stack.push(Value::int(4));
        vm.data_stack.push(Value::int(4));

        vm.exec_top_frame().unwrap();

        assert_eq!(vm.data_stack.pop().unwrap(), Value::int(32));
    }

    #[test]
    fn test_jump_less() {
        let mut obj = init_code_obj_with_pool(
            Bytecode::new(vec![
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(0), // 1
                Instr::LoadLit(1), // 2
                Instr::JumpLt(0),  // True (1 < 2)
                Instr::BinOp(BinOp::Add),
                Instr::BinOp(BinOp::Mul),
            ]),
            vec![Value::int(1), Value::int(2), Value::int(4)],
        );
        obj.labels.insert(0, 7);
        let mut vm = init_test_vm(&obj);

        vm.data_stack.push(Value::int(1));
        vm.data_stack.push(Value::int(2));
        vm.data_stack.push(Value::int(4));
        vm.data_stack.push(Value::int(4));

        vm.exec_top_frame().unwrap();

        assert_eq!(vm.data_stack.pop().unwrap(), Value::int(16));
    }
}
