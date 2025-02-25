use std::cmp::{Ord, Ordering};
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Shl, Shr, Sub};

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::bytecode::{BinOp, Bytecode, Instr, UnaryOp};
use crate::{Hash, HASH_SIZE};

const STACK_CAP: usize = 256;

#[derive(Debug)]
struct Vm {
    data_stack: Vec<Value>,
    call_stack: Vec<StackFrame>,
    data_stack_cap: usize,
    call_stack_cap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeObject {
    litpool: Vec<Value>,
    argcount: usize,
    is_void: bool,
    localnames: Vec<String>,
    /// Map from label index to an offset in the bytecode
    labels: HashMap<usize, usize>,

    code: Bytecode,
}

/// An execution context for a code object
#[derive(Debug, Clone)]
struct StackFrame {
    code_obj: CodeObject, // TODO: make this a reference
    // They all start uninitialized.
    // ... Or it starts empty.
    // Will need to think
    // Also consider making it a BTreeMap (with a max cap)
    locals: HashMap<String, Value>,
    instruction: usize,
    // maybe add some debug info like a name
}

/// A value that can be on the stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    I32(i32),
    String(String),
    Bool(bool),
    Hash(Hash),
}

impl Value {
    pub fn int(i: i32) -> Value {
        Value::I32(i)
    }

    pub fn string(s: &str) -> Value {
        Value::String(s.to_string())
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Some(x.cmp(&y)),
            _ => panic!("cannot compare non-integer values"),
        }
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => x.cmp(&y),
            _ => panic!("cannot compare non-integer values"),
        }
    }
}

impl Vm {
    pub fn new() -> Vm {
        Vm {
            data_stack: Vec::new(),
            call_stack: Vec::new(),
            data_stack_cap: STACK_CAP,
            call_stack_cap: STACK_CAP,
        }
    }

    /// Execute the frame at the top of the call stack.
    pub fn exec_top_frame(&mut self) -> Result<()> {
        let bytecode = self.call_stack.last().unwrap().code_obj.code.clone();
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

            Instr::LoadFunc(hash) => {
                stack.push(Value::Hash(*hash));
            }

            Instr::Call => {
                // Pop hash from stack
                if let Some(Value::Hash(hash)) = stack.pop() {
                    // find the right code object by looking up the hash in the database
                    // construct a new stackframe
                    // push to the vm.call_stack
                } else {
                    bail!("cannot call function: function hash not present");
                }
            }
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

impl CodeObject {
    pub fn hash(&self) -> Result<Hash> {
        let obj = rmp_serde::to_vec(&self)?;
        let mut hasher = Sha512::new();
        hasher.update(obj);
        (&hasher.finalize().to_vec()[0..HASH_SIZE])
            .try_into()
            .map_err(|_| anyhow!("failed to hash CodeObject"))
    }

    pub fn hash_str(&self) -> Result<String> {
        let hash = self.hash()?;
        Ok(format!("0x{}", hex::encode(hash)))
    }
}

impl Add for Value {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x + y),
            (Value::String(x), Value::String(y)) => Value::String(x + &y),
            _ => panic!("cannot add values of different type"),
        }
    }
}

impl Sub for Value {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x - y),
            _ => panic!("failed to perform sub"),
        }
    }
}

impl Mul for Value {
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

impl Div for Value {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x / y),
            _ => panic!("failed to perform div"),
        }
    }
}

impl Rem for Value {
    type Output = Self;

    fn rem(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x % y),
            _ => panic!("failed to perform rem"),
        }
    }
}

impl Shl for Value {
    type Output = Self;

    fn shl(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x << y),
            _ => panic!("failed to perform shl"),
        }
    }
}

impl Shr for Value {
    type Output = Self;

    fn shr(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(x >> y),
            _ => panic!("failed to perform shr"),
        }
    }
}

impl Neg for Value {
    type Output = Self;

    fn neg(self) -> Self {
        match self {
            Value::I32(x) => Value::I32(-x),
            _ => panic!("failed to perform neg"),
        }
    }
}

impl Not for Value {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Value::Bool(x) => Value::Bool(!x),
            _ => panic!("failed to perform not"),
        }
    }
}

impl Value {
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

    use rand::{distr::Alphanumeric, Rng};

    pub fn init_code_obj(code: Bytecode) -> CodeObject {
        CodeObject {
            litpool: vec![Value::int(5), Value::string("hello")],
            argcount: 2, // x and y
            is_void: false,
            labels: HashMap::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    pub fn init_nondet_code_obj(code: Bytecode) -> CodeObject {
        let s: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        CodeObject {
            litpool: vec![Value::int(5), Value::String(s)],
            argcount: 2, // x and y
            is_void: false,
            labels: HashMap::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_code_obj_with_pool(code: Bytecode, litpool: Vec<Value>) -> CodeObject {
        CodeObject {
            litpool,
            argcount: 2, // x and y
            is_void: false,
            labels: HashMap::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_test_vm(code_obj: &CodeObject) -> Vm {
        let mut vm = Vm::new();

        let frame = StackFrame {
            code_obj: code_obj.to_owned(),
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

    #[test]
    fn test_hash_code_obj() {
        let obj = init_code_obj(Bytecode::default());
        println!("{:?}", obj.hash_str());
    }
}
