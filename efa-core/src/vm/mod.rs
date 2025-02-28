use std::cmp::{Ord, Ordering};
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Shl, Shr, Sub};
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::bytecode::{BinOp, Bytecode, Instr, UnaryOp};
use crate::db::Database;
use crate::{Hash, HASH_SIZE};

const STACK_CAP: usize = 256;

#[derive(Debug)]
struct Vm {
    call_stack: Vec<StackFrame>,
    data_stack_cap: usize,
    call_stack_cap: usize,
    db: Database,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeObject {
    pub(crate) litpool: Vec<Value>,
    pub(crate) argcount: usize,
    pub(crate) is_void: bool,
    // TODO: change to be num_locals? then the stack frame locals could be vec<value>
    // Worse debuggability
    pub(crate) localnames: Vec<String>,
    /// Map from label index to an offset in the bytecode
    pub(crate) labels: Vec<usize>,

    pub(crate) code: Bytecode,
}

/// An execution context for a code object
#[derive(Debug, Clone)]
struct StackFrame {
    code_obj: CodeObject, // TODO: make this a reference
    // They all start uninitialized.
    // ... Or it starts empty.
    // Will need to think
    // Also consider making it a BTreeMap (with a max cap)
    stack: Vec<Value>,
    locals: HashMap<String, Value>,
    instruction: usize,
    // maybe add some debug info like a name
}

enum Return {
    Value(Value),
    Void,
    None,
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
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => x.cmp(y),
            _ => panic!("cannot compare non-integer values"),
        }
    }
}

impl Vm {
    pub fn new() -> Result<Vm> {
        Ok(Vm {
            call_stack: Vec::new(),
            data_stack_cap: STACK_CAP,
            call_stack_cap: STACK_CAP,
            db: Database::temp()?,
        })
    }

    pub fn initialize<P: AsRef<Path>>(path: P) -> Result<Vm> {
        Ok(Vm {
            call_stack: Vec::new(),
            data_stack_cap: STACK_CAP,
            call_stack_cap: STACK_CAP,
            db: Database::open(path)?,
        })
    }

    /// Return exit code
    pub fn run_main_function(&mut self, code: &CodeObject) -> Result<i32> {
        let main = StackFrame {
            code_obj: code.clone(),
            stack: Vec::new(),
            locals: HashMap::new(),
            instruction: 0,
        };
        self.call_stack.push(main);
        self.exec(false)
    }

    /// Run the given frame and return the final state of the frame.
    /// Mainly used for debugging.
    fn run_frame(&mut self, frame: StackFrame) -> Result<StackFrame> {
        self.call_stack.push(frame);
        self.exec(true)?;
        Ok(self.call_stack.last().unwrap().clone())
    }

    /// With debug=true, the final frame will stay on the call stack.
    fn exec(&mut self, debug: bool) -> Result<i32> {
        let mut status_code = 0;

        while !self.call_stack.is_empty() {
            let call_depth = self.call_stack.len();
            let frame = &mut self.call_stack[call_depth - 1];
            let stack = &mut frame.stack;
            if frame.instruction >= frame.code_obj.code.len() {
                // Handle the case of a forgotten return statement
                break;
            }
            let instr = frame.code_obj.code[frame.instruction].clone();
            let mut next_instr_ptr = frame.instruction + 1; // Default

            let mut return_value: Return = Return::None;
            let mut next_frame: Option<StackFrame> = None;

            match instr {
                Instr::LoadArg(i) => {
                    if i >= frame.code_obj.argcount {
                        bail!("argument index {i} out of bounds");
                    }
                    let arg_name = &frame.code_obj.localnames[i];
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
                    stack.push(frame.code_obj.litpool[i].clone());
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
                    stack.push(Value::Hash(hash));
                }

                Instr::LoadDyn(name) => {
                    let (hash, _) = self.db.get_code_object_by_name(&name)?;
                    stack.push(Value::Hash(hash));
                }

                Instr::Call => {
                    // Pop hash from stack
                    if let Some(Value::Hash(hash)) = stack.pop() {
                        // Find the right code object by looking up the hash in the database
                        let code_obj = self.db.get_code_object(&hash)?;

                        // Set up parameters
                        let params: Result<_> = code_obj
                            .localnames
                            .iter()
                            .take(code_obj.argcount)
                            .map(|name| {
                                if stack.is_empty() {
                                    bail!("not enough arguments on stack to call function with arity {}", code_obj.argcount);
                                }
                                Ok((name.to_owned(), stack.pop().unwrap()))
                            }).collect();

                        // Construct a new stackframe
                        let new_frame = StackFrame {
                            stack: Vec::new(),
                            code_obj,
                            locals: params?,
                            instruction: 0,
                        };

                        next_frame = Some(new_frame);
                    } else {
                        bail!("cannot call function: function hash not present");
                    }
                }

                // TODO: reduce code duplication with Call
                Instr::CallSelf => {
                    let code_obj = frame.code_obj.clone();

                    // Set up parameters
                    let params: Result<_> = code_obj
                        .localnames
                        .iter()
                        .take(code_obj.argcount)
                        .map(|name| {
                            if stack.is_empty() {
                                bail!(
                                    "not enough arguments on stack to call function with arity {}",
                                    code_obj.argcount
                                );
                            }
                            Ok((name.to_owned(), stack.pop().unwrap()))
                        })
                        .collect();

                    let new_frame = StackFrame {
                        stack: Vec::new(),
                        code_obj: frame.code_obj.clone(),
                        locals: params?,
                        instruction: 0,
                    };

                    next_frame = Some(new_frame);
                }

                Instr::Return => {
                    // Return value is whatever is on the top of the stack
                    // If we have `return x`, then we (the compiler) LOAD x to push it to the top of the stack
                    if frame.code_obj.is_void {
                        return_value = Return::Void;
                    } else {
                        // Get the return value from the top of current frame's stack
                        if stack.is_empty() {
                            bail!("non-void function requires a return value on the stack");
                        } else {
                            return_value = Return::Value(stack.pop().unwrap());
                        }
                    };
                }

                Instr::Jump(label) => next_instr_ptr = frame.code_obj.labels[label],

                Instr::JumpT(label) => {
                    if stack.is_empty() {
                        bail!("cannot perform jump: stack underflow");
                    }

                    let top = stack.pop().unwrap();

                    if let Value::Bool(true) = top {
                        next_instr_ptr = frame
                            .code_obj
                            .labels
                            .get(label)
                            .copied()
                            .ok_or_else(|| anyhow!("label {} does not exist", label))?;
                    }
                }

                Instr::JumpF(label) => {
                    if stack.is_empty() {
                        bail!("cannot perform jump: stack underflow");
                    }

                    let top = stack.pop().unwrap();

                    if let Value::Bool(false) = top {
                        next_instr_ptr = frame.code_obj.labels[label];
                    }
                }

                Instr::JumpEq(label) => {
                    if stack.len() < 2 {
                        bail!("cannot perform comparison: stack underflow");
                    }

                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();

                    if lhs == rhs {
                        next_instr_ptr = frame.code_obj.labels[label];
                    }
                }
                Instr::JumpGt(label) => {
                    if stack.len() < 2 {
                        bail!("cannot perform comparison: stack underflow");
                    }

                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();

                    if lhs > rhs {
                        next_instr_ptr = frame.code_obj.labels[label];
                    }
                }
                Instr::JumpGe(label) => {
                    if stack.len() < 2 {
                        bail!("cannot perform comparison: stack underflow");
                    }

                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();

                    if lhs >= rhs {
                        next_instr_ptr = frame.code_obj.labels[label];
                    }
                }
                Instr::JumpLt(label) => {
                    if stack.len() < 2 {
                        bail!("cannot perform comparison: stack underflow");
                    }

                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();

                    if lhs < rhs {
                        next_instr_ptr = frame.code_obj.labels[label];
                    }
                }
                Instr::JumpLe(label) => {
                    if stack.len() < 2 {
                        bail!("cannot perform comparison: stack underflow");
                    }

                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();

                    if lhs <= rhs {
                        next_instr_ptr = frame.code_obj.labels[label];
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
                        BinOp::Eq => stack.push(Value::Bool(lhs == rhs)),
                        BinOp::Or => stack.push(lhs.or(rhs)),
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

                Instr::Dbg => {
                    let tos = stack.pop().unwrap();
                    dbg!(tos);
                }
                Instr::Nop => {}
            }

            // Update program counter for this frame
            frame.instruction = next_instr_ptr;

            // If the instruction was a call, then update the stack frame
            if let Some(frame) = next_frame {
                self.call_stack.push(frame);
            }

            // Handle a return
            match return_value {
                Return::Value(val) => {
                    // If the main function returns
                    if call_depth == 1 {
                        // Note: this case keeps the main function's frame around
                        if let Value::I32(code) = val {
                            status_code = code;
                        } else {
                            bail!("main function can only return integers");
                        }
                        break;
                    }

                    self.call_stack.pop();
                    // Push the returning function's return value onto the caller's stack
                    self.call_stack[call_depth - 2].stack.push(val);
                }
                Return::Void => {
                    self.call_stack.pop();
                }
                // Instruction was not a return
                Return::None => {}
            }
        }

        if !debug {
            self.call_stack.pop();
        }

        Ok(status_code)
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
            (Value::Bool(x), Value::Bool(y)) => Value::Bool(x && y),
            _ => panic!("failed to perform and"),
        }
    }

    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Value::I32(x), Value::I32(y)) => Value::I32(((x != 0) && (y != 0)) as i32),
            (Value::String(s), Value::I32(x)) | (Value::I32(x), Value::String(s)) => {
                Value::I32((!s.is_empty() && (x != 0)) as i32)
            }
            (Value::Bool(x), Value::Bool(y)) => Value::Bool(x || y),
            _ => panic!("failed to perform or"),
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
            labels: Vec::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_frame(code: Bytecode) -> StackFrame {
        let code_obj = init_code_obj(code);
        StackFrame {
            code_obj,
            stack: Vec::new(),
            locals: HashMap::from([
                ("x".into(), Value::int(10)),
                ("y".into(), Value::string("ok")),
                ("z".into(), Value::int(64)),
            ]),
            instruction: 0,
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
            labels: Vec::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_code_obj_with_pool(code: Bytecode, litpool: Vec<Value>) -> CodeObject {
        CodeObject {
            litpool,
            argcount: 2, // x and y
            is_void: false,
            labels: Vec::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_test_vm(code_obj: &CodeObject) -> Vm {
        let mut vm = Vm::new().unwrap();

        let frame = StackFrame {
            stack: Vec::new(),
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
        let main = init_frame(bytecode![Instr::LoadArg(1), Instr::LoadArg(0)]);
        let mut vm = Vm::new().unwrap();

        let mut frame = vm.run_frame(main).unwrap();
        let tos = frame.stack.pop().unwrap();
        assert!(matches!(tos, Value::I32(10)));
        let tos = frame.stack.pop().unwrap();
        assert!(matches!(tos, Value::String(ref s) if s == "ok"));
    }

    #[test]
    fn test_load_local() {
        let main = init_frame(bytecode![Instr::LoadLocal(0)]);
        let mut vm = Vm::new().unwrap();

        let mut frame = vm.run_frame(main).unwrap();
        let tos = frame.stack.pop().unwrap();
        assert!(matches!(tos, Value::I32(64)));
    }

    #[test]
    fn test_load_lit() {
        let main = init_frame(bytecode![Instr::LoadLit(1)]);
        let mut vm = Vm::new().unwrap();

        let mut frame = vm.run_frame(main).unwrap();
        let tos = frame.stack.pop().unwrap();
        assert!(matches!(tos, Value::String(ref s) if s == "hello"));
    }

    #[test]
    fn test_store_local() {
        let mut main = init_frame(bytecode![Instr::StoreLocal(0)]);
        let mut vm = Vm::new().unwrap();

        let v = Value::I32(100);
        main.stack.push(v.clone());

        let frame = vm.run_frame(main).unwrap();

        // Check
        assert_eq!(frame.locals.get("z".into()).unwrap().to_owned(), v);
    }

    #[test]
    fn test_ops() {
        let mut main = init_frame(bytecode![
            Instr::BinOp(BinOp::Add),
            Instr::BinOp(BinOp::Mul),
            Instr::BinOp(BinOp::Mod),
            Instr::BinOp(BinOp::Sub),
            Instr::UnaryOp(UnaryOp::Neg)
        ]);
        let mut vm = Vm::new().unwrap();

        main.stack.push(Value::int(5));
        main.stack.push(Value::int(4));
        main.stack.push(Value::int(6));
        main.stack.push(Value::int(3));
        main.stack.push(Value::int(2));

        let mut frame = vm.run_frame(main).unwrap();

        // 3 + 2
        // 6 * 5
        // 4 % 30
        // 5 - 4
        // -1
        assert_eq!(
            frame.stack.pop().unwrap(),
            Value::int(-(5 - (4 % (6 * (3 + 2)))))
        );
    }

    #[test]
    fn test_jump() {
        let mut main = init_frame(bytecode![
            Instr::Jump(0),
            Instr::BinOp(BinOp::Add),
            Instr::BinOp(BinOp::Mul)
        ]);
        main.code_obj.labels.push(2);
        let mut vm = Vm::new().unwrap();

        main.stack.push(Value::int(5));
        main.stack.push(Value::int(4));

        let mut frame = vm.run_frame(main).unwrap();

        assert_eq!(frame.stack.pop().unwrap(), Value::int(20));
    }

    #[test]
    fn test_jump_greater() {
        let mut code_obj = init_code_obj_with_pool(
            bytecode![
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(0), // 1
                Instr::LoadLit(1), // 2
                Instr::JumpGt(0),  // False (since 1 > 2 is false)
                Instr::BinOp(BinOp::Add),
                Instr::BinOp(BinOp::Mul)
            ],
            vec![Value::int(1), Value::int(2), Value::int(4)],
        );
        code_obj.labels.push(7);

        let main = StackFrame {
            code_obj,
            stack: vec![Value::int(1), Value::int(2), Value::int(4), Value::int(4)],
            instruction: 0,
            locals: HashMap::new(),
        };

        let mut vm = Vm::new().unwrap();

        let mut frame = vm.run_frame(main).unwrap();
        assert_eq!(frame.stack.pop().unwrap(), Value::int(32));
    }

    #[test]
    fn test_jump_less() {
        let mut code_obj = init_code_obj_with_pool(
            bytecode![
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(2), // 4
                Instr::LoadLit(0), // 1
                Instr::LoadLit(1), // 2
                Instr::JumpLt(0),  // True (1 < 2)
                Instr::BinOp(BinOp::Add),
                Instr::BinOp(BinOp::Mul)
            ],
            vec![Value::int(1), Value::int(2), Value::int(4)],
        );
        code_obj.labels.push(7);
        let main = StackFrame {
            code_obj,
            stack: vec![Value::int(1), Value::int(2), Value::int(4), Value::int(4)],
            instruction: 0,
            locals: HashMap::new(),
        };

        let mut vm = Vm::new().unwrap();

        let mut frame = vm.run_frame(main).unwrap();
        assert_eq!(frame.stack.pop().unwrap(), Value::int(16));
    }

    #[test]
    fn test_hash_code_obj() {
        let obj = init_code_obj(Bytecode::default());
        println!("{:?}", obj.hash_str());
    }

    /* Testing function calls */
    /*
     1. Create a new in-mem db / vm
     2. Create a code object B to be called and insert it into the db
     3. Create a code object A to call B
     4. Run A
    */

    #[test]
    fn test_main_return_code() {
        let mut vm = Vm::new().unwrap();

        let func_b = CodeObject {
            litpool: vec![Value::int(4), Value::int(3)],
            argcount: 0,
            is_void: false,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![
                Instr::LoadLit(0), // 4
                Instr::LoadLit(1), // 3
                Instr::BinOp(BinOp::Add),
                Instr::Return
            ],
        };

        let hash = vm
            .db
            .insert_code_object_with_name(&func_b, "func_b")
            .unwrap();

        let func_a = CodeObject {
            litpool: vec![Value::I32(10)],
            argcount: 0,
            is_void: false,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![
                Instr::LoadFunc(hash),
                Instr::Call,
                Instr::LoadLit(0),
                Instr::BinOp(BinOp::Mul),
                Instr::Return
            ],
        };

        let code = vm.run_main_function(&func_a).unwrap();

        assert_eq!(code, 70);
    }

    #[test]
    fn test_void_funccall() {
        let mut vm = Vm::new().unwrap();

        let func_b = CodeObject {
            litpool: vec![Value::int(4), Value::int(3)],
            argcount: 0,
            is_void: true,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![
                Instr::LoadLit(0), // 4
                Instr::LoadLit(1), // 3
                Instr::BinOp(BinOp::Add),
                Instr::Return
            ],
        };

        let hash = vm
            .db
            .insert_code_object_with_name(&func_b, "func_b")
            .unwrap();

        let func_a = CodeObject {
            litpool: vec![],
            argcount: 0,
            is_void: true,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![Instr::LoadFunc(hash), Instr::Call, Instr::Return],
        };

        let code = vm.run_main_function(&func_a).unwrap();

        assert_eq!(code, 0);
    }

    #[test]
    fn test_main_returns() {
        let mut vm = Vm::new().unwrap();
        let func = CodeObject {
            litpool: vec![],
            argcount: 0,
            is_void: false,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![Instr::Return],
        };
        assert!(vm.run_main_function(&func).is_err());

        let func = CodeObject {
            litpool: vec![Value::string("break")],
            argcount: 0,
            is_void: false,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![Instr::LoadLit(0), Instr::Return],
        };

        assert!(vm.run_main_function(&func).is_err());

        let func = CodeObject {
            litpool: vec![Value::I32(0)],
            argcount: 0,
            is_void: false,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![Instr::LoadLit(0), Instr::Return],
        };

        assert_eq!(vm.run_main_function(&func).unwrap(), 0);
    }

    #[test]
    fn test_fib() {
        let mut vm = Vm::new().unwrap();
        let fib = CodeObject {
            litpool: vec![Value::I32(0), Value::I32(1), Value::I32(2)],
            argcount: 1,
            is_void: false,
            localnames: vec!["n".into()],
            labels: vec![18],
            code: bytecode![
                Instr::LoadArg(0),       // load n
                Instr::LoadLit(0),       // load 0
                Instr::BinOp(BinOp::Eq), // push n == 0
                Instr::LoadArg(0),       // load n
                Instr::LoadLit(1),       // load 1
                Instr::BinOp(BinOp::Eq), // push n == 1
                Instr::BinOp(BinOp::Or), // push (n == 0) || (n == 1)
                Instr::JumpT(0),         // jump to label 0
                // fib(n-1)
                Instr::LoadArg(0),
                Instr::LoadLit(1),
                Instr::BinOp(BinOp::Sub),
                Instr::CallSelf,
                // fib(n-2)
                Instr::LoadArg(0),
                Instr::LoadLit(2),
                Instr::BinOp(BinOp::Sub),
                Instr::CallSelf,
                // fib(n-1) + fib(n-2)
                Instr::BinOp(BinOp::Add),
                Instr::Return,
                // Label 0 (line 18)
                Instr::LoadArg(0), // push n
                Instr::Return
            ],
        };

        let hash = vm.db.insert_code_object_with_name(&fib, "fib").unwrap();

        let mut f = |n: i32| -> i32 {
            let main = CodeObject {
                litpool: vec![Value::I32(n)],
                argcount: 0,
                is_void: false,
                localnames: vec![],
                labels: Vec::new(),
                code: bytecode![
                    Instr::LoadLit(0),
                    Instr::LoadFunc(hash),
                    Instr::Call,
                    Instr::Return
                ],
            };
            vm.run_main_function(&main).unwrap()
        };

        assert_eq!(f(10), 55);
        assert_eq!(f(15), 610);
        assert_eq!(f(25), 75025);
    }
}
