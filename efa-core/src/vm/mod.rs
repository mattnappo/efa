use std::cmp::{Ord, Ordering};
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Shl, Shr, Sub};
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::bytecode::{BinOp, Bytecode, Instr, UnaryOp};
use crate::db::Database;
use crate::{hash_from_vec, Hash, HASH_SIZE};

const STACK_CAP: usize = 256;

#[derive(Debug)]
pub struct Vm {
    call_stack: Vec<StackFrame>,
    data_stack_cap: usize,
    call_stack_cap: usize,
    pub db: Database, // TODO: should not be pub
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeObject {
    pub(crate) litpool: Vec<Value>,
    pub(crate) argcount: usize,
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

/// A value that can be on the stack.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Value {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    Isize(isize),
    Usize(usize),

    F32(f32),
    F64(f64),

    Char(char),
    Bool(bool),

    Hash(Hash),
    String(String), // TODO: make a borrowed version?

    Container(Vec<Value>),
}

impl Value {
    pub fn int(i: i32) -> Value {
        Value::I32(i)
    }

    pub fn string(s: &str) -> Value {
        Value::String(s.to_string())
    }

    pub fn hash(hash: Vec<u8>) -> Result<Value> {
        let trunc = hash_from_vec(hash)?;
        Ok(Value::Hash(trunc))
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::I8(x), Value::I8(y)) => Some(x.cmp(y)),
            (Value::U8(x), Value::U8(y)) => Some(x.cmp(y)),
            (Value::I16(x), Value::I16(y)) => Some(x.cmp(y)),
            (Value::U16(x), Value::U16(y)) => Some(x.cmp(y)),
            (Value::I32(x), Value::I32(y)) => Some(x.cmp(y)),
            (Value::U32(x), Value::U32(y)) => Some(x.cmp(y)),
            (Value::I64(x), Value::I64(y)) => Some(x.cmp(y)),
            (Value::U64(x), Value::U64(y)) => Some(x.cmp(y)),
            (Value::I128(x), Value::I128(y)) => Some(x.cmp(y)),
            (Value::U128(x), Value::U128(y)) => Some(x.cmp(y)),
            (Value::Isize(x), Value::Isize(y)) => Some(x.cmp(y)),
            (Value::Usize(x), Value::Usize(y)) => Some(x.cmp(y)),
            (Value::Char(x), Value::Char(y)) => Some(x.cmp(y)),
            (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
            (Value::Hash(x), Value::Hash(y)) => Some(x.cmp(y)),
            (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
            e => panic!("cannot compare {e:?}"),
        }
    }
}

impl Vm {
    /// Create an in-memory VM
    pub fn new() -> Result<Vm> {
        Ok(Vm {
            call_stack: Vec::new(),
            data_stack_cap: STACK_CAP,
            call_stack_cap: STACK_CAP,
            db: Database::temp()?,
        })
    }

    /// Start a VM from an existing database
    pub fn initialize<P: AsRef<Path>>(path: P) -> Result<Vm> {
        Ok(Vm {
            call_stack: Vec::new(),
            data_stack_cap: STACK_CAP,
            call_stack_cap: STACK_CAP,
            db: Database::open(path)?,
        })
    }

    /// Create a new VM with a new persistent database
    pub fn persistent<P: AsRef<Path>>(path: P) -> Result<Vm> {
        Ok(Vm {
            call_stack: Vec::new(),
            data_stack_cap: STACK_CAP,
            call_stack_cap: STACK_CAP,
            db: Database::new(path)?,
        })
    }

    /// Return exit code
    /// TODO: does not handle locals yet
    pub fn run_main_function(&mut self) -> Result<i32> {
        let (_, code_obj) = self.db.get_main_object()?;

        let main = StackFrame {
            code_obj,
            stack: Vec::new(),
            locals: HashMap::new(),
            instruction: 0,
        };
        self.call_stack.push(main);
        self.exec(false)
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

            let mut return_value = None;
            let mut next_frame: Option<StackFrame> = None;
            //println!("{instr:?}");
            match instr {
                Instr::LoadArg(i) => {
                    if i >= frame.code_obj.argcount {
                        bail!("argument index {i} out of bounds");
                    }
                    let arg_name = &frame.code_obj.localnames[i];

                    let val = frame.locals.get(arg_name).ok_or_else(|| {
                        anyhow!("argument '{arg_name}' with index {i} is out of bounds")
                    })?;
                    stack.push(val.clone());
                }
                Instr::LoadLocal(i) => {
                    let k = i + frame.code_obj.argcount;
                    if k >= frame.code_obj.localnames.len() {
                        bail!("local index {k} out of bounds");
                    }
                    let arg_name = &frame.code_obj.localnames[k];
                    //dbg!(&i);
                    //dbg!(&k);
                    //dbg!(&arg_name);
                    //dbg!(&frame.locals);
                    //dbg!(&frame.code_obj.localnames);
                    let val = frame.locals.get(arg_name).ok_or_else(|| {
                        anyhow!("local '{arg_name}' with index {i} is out of bounds")
                    })?;
                    stack.push(val.clone());
                }
                Instr::LoadLit(i) => {
                    // TODO: throw err with out of bounds
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
                Instr::Dup => {
                    stack.push(stack.iter().last().unwrap().clone());
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

                        // println!("argc = {:?}", code_obj.argcount);
                        // println!("params = {:?}", params);

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
                    return_value = Some(None);
                }
                Instr::ReturnVal => {
                    // Return value is whatever is on the top of the stack
                    // If we have `return x`, then we (the compiler) LOAD x to push it to the top of the stack
                    // Get the return value from the top of current frame's stack
                    if stack.is_empty() {
                        bail!("non-void function requires a return value on the stack");
                    } else {
                        return_value = Some(Some(stack.pop().unwrap()));
                    }
                }

                Instr::Jump(label) => next_instr_ptr = frame.code_obj.labels[label],

                Instr::JumpT(label) => {
                    if stack.is_empty() {
                        bail!("cannot perform jump: stack underflow");
                    }

                    let top = stack.pop().unwrap();

                    if let Value::Bool(true) = top {
                        next_instr_ptr =
                            frame.code_obj.labels.get(label).copied().ok_or_else(
                                || anyhow!("label {} does not exist", label),
                            )?;
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
                Instr::JumpNe(label) => {
                    if stack.len() < 2 {
                        bail!("cannot perform comparison: stack underflow");
                    }

                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();

                    if lhs != rhs {
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
                    let tos = stack.last().ok_or_else(|| {
                        anyhow!("stack underflow: cannot 'dbg' with empty stack")
                    })?;
                    println!("{tos:?} ");
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
                Some(Some(val)) => {
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
                Some(None) => {
                    self.call_stack.pop();
                }
                // Instruction was not a return
                None => {}
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
        (&hasher.finalize().to_vec()[..HASH_SIZE])
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
            // Signed integers
            (Value::I8(x), Value::I8(y)) => Value::I8(x + y),
            (Value::I16(x), Value::I16(y)) => Value::I16(x + y),
            (Value::I32(x), Value::I32(y)) => Value::I32(x + y),
            (Value::I64(x), Value::I64(y)) => Value::I64(x + y),
            (Value::I128(x), Value::I128(y)) => Value::I128(x + y),
            (Value::Isize(x), Value::Isize(y)) => Value::Isize(x + y),

            // Unsigned integers
            (Value::U8(x), Value::U8(y)) => Value::U8(x + y),
            (Value::U16(x), Value::U16(y)) => Value::U16(x + y),
            (Value::U32(x), Value::U32(y)) => Value::U32(x + y),
            (Value::U64(x), Value::U64(y)) => Value::U64(x + y),
            (Value::U128(x), Value::U128(y)) => Value::U128(x + y),
            (Value::Usize(x), Value::Usize(y)) => Value::Usize(x + y),

            // Floats
            (Value::F32(x), Value::F32(y)) => Value::F32(x + y),
            (Value::F64(x), Value::F64(y)) => Value::F64(x + y),

            // Strings
            (Value::String(x), Value::String(y)) => Value::String(x + &y),

            _ => panic!("cannot add values of different types"),
        }
    }
}

impl Sub for Value {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        match (self, other) {
            // Signed integers
            (Value::I8(x), Value::I8(y)) => Value::I8(x - y),
            (Value::I16(x), Value::I16(y)) => Value::I16(x - y),
            (Value::I32(x), Value::I32(y)) => Value::I32(x - y),
            (Value::I64(x), Value::I64(y)) => Value::I64(x - y),
            (Value::I128(x), Value::I128(y)) => Value::I128(x - y),
            (Value::Isize(x), Value::Isize(y)) => Value::Isize(x - y),

            // Unsigned integers
            (Value::U8(x), Value::U8(y)) => Value::U8(x - y),
            (Value::U16(x), Value::U16(y)) => Value::U16(x - y),
            (Value::U32(x), Value::U32(y)) => Value::U32(x - y),
            (Value::U64(x), Value::U64(y)) => Value::U64(x - y),
            (Value::U128(x), Value::U128(y)) => Value::U128(x - y),
            (Value::Usize(x), Value::Usize(y)) => Value::Usize(x - y),

            // Floats
            (Value::F32(x), Value::F32(y)) => Value::F32(x - y),
            (Value::F64(x), Value::F64(y)) => Value::F64(x - y),

            _ => panic!("cannot subtract values of different types"),
        }
    }
}

impl Mul for Value {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        match (self, other) {
            // Signed integers
            (Value::I8(x), Value::I8(y)) => Value::I8(x * y),
            (Value::I16(x), Value::I16(y)) => Value::I16(x * y),
            (Value::I32(x), Value::I32(y)) => Value::I32(x * y),
            (Value::I64(x), Value::I64(y)) => Value::I64(x * y),
            (Value::I128(x), Value::I128(y)) => Value::I128(x * y),
            (Value::Isize(x), Value::Isize(y)) => Value::Isize(x * y),

            // Unsigned integers
            (Value::U8(x), Value::U8(y)) => Value::U8(x * y),
            (Value::U16(x), Value::U16(y)) => Value::U16(x * y),
            (Value::U32(x), Value::U32(y)) => Value::U32(x * y),
            (Value::U64(x), Value::U64(y)) => Value::U64(x * y),
            (Value::U128(x), Value::U128(y)) => Value::U128(x * y),
            (Value::Usize(x), Value::Usize(y)) => Value::Usize(x * y),

            // Floats
            (Value::F32(x), Value::F32(y)) => Value::F32(x * y),
            (Value::F64(x), Value::F64(y)) => Value::F64(x * y),

            _ => panic!("cannot multiply values of different types"),
        }
    }
}

impl Div for Value {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        match (self, other) {
            // Signed integers
            (Value::I8(x), Value::I8(y)) => Value::I8(x / y),
            (Value::I16(x), Value::I16(y)) => Value::I16(x / y),
            (Value::I32(x), Value::I32(y)) => Value::I32(x / y),
            (Value::I64(x), Value::I64(y)) => Value::I64(x / y),
            (Value::I128(x), Value::I128(y)) => Value::I128(x / y),
            (Value::Isize(x), Value::Isize(y)) => Value::Isize(x / y),

            // Unsigned integers
            (Value::U8(x), Value::U8(y)) => Value::U8(x / y),
            (Value::U16(x), Value::U16(y)) => Value::U16(x / y),
            (Value::U32(x), Value::U32(y)) => Value::U32(x / y),
            (Value::U64(x), Value::U64(y)) => Value::U64(x / y),
            (Value::U128(x), Value::U128(y)) => Value::U128(x / y),
            (Value::Usize(x), Value::Usize(y)) => Value::Usize(x / y),

            // Floats
            (Value::F32(x), Value::F32(y)) => Value::F32(x / y),
            (Value::F64(x), Value::F64(y)) => Value::F64(x / y),

            _ => panic!("cannot divide values of different types"),
        }
    }
}

impl Rem for Value {
    type Output = Self;

    fn rem(self, other: Self) -> Self {
        match (self, other) {
            // Signed integers
            (Value::I8(x), Value::I8(y)) => Value::I8(x % y),
            (Value::I16(x), Value::I16(y)) => Value::I16(x % y),
            (Value::I32(x), Value::I32(y)) => Value::I32(x % y),
            (Value::I64(x), Value::I64(y)) => Value::I64(x % y),
            (Value::I128(x), Value::I128(y)) => Value::I128(x % y),
            (Value::Isize(x), Value::Isize(y)) => Value::Isize(x % y),

            // Unsigned integers
            (Value::U8(x), Value::U8(y)) => Value::U8(x % y),
            (Value::U16(x), Value::U16(y)) => Value::U16(x % y),
            (Value::U32(x), Value::U32(y)) => Value::U32(x % y),
            (Value::U64(x), Value::U64(y)) => Value::U64(x % y),
            (Value::U128(x), Value::U128(y)) => Value::U128(x % y),
            (Value::Usize(x), Value::Usize(y)) => Value::Usize(x % y),

            // Floats
            (Value::F32(x), Value::F32(y)) => Value::F32(x % y),
            (Value::F64(x), Value::F64(y)) => Value::F64(x % y),

            _ => panic!("cannot perform modulo on values of different types"),
        }
    }
}

impl Shl for Value {
    type Output = Self;

    fn shl(self, other: Self) -> Self {
        match (self, other) {
            // Signed integers
            (Value::I8(x), Value::I8(y)) => Value::I8(x << y),
            (Value::I16(x), Value::I16(y)) => Value::I16(x << y),
            (Value::I32(x), Value::I32(y)) => Value::I32(x << y),
            (Value::I64(x), Value::I64(y)) => Value::I64(x << y),
            (Value::I128(x), Value::I128(y)) => Value::I128(x << y),
            (Value::Isize(x), Value::Isize(y)) => Value::Isize(x << y),

            // Unsigned integers
            (Value::U8(x), Value::U8(y)) => Value::U8(x << y),
            (Value::U16(x), Value::U16(y)) => Value::U16(x << y),
            (Value::U32(x), Value::U32(y)) => Value::U32(x << y),
            (Value::U64(x), Value::U64(y)) => Value::U64(x << y),
            (Value::U128(x), Value::U128(y)) => Value::U128(x << y),
            (Value::Usize(x), Value::Usize(y)) => Value::Usize(x << y),

            _ => panic!("cannot perform left shift on values of different types"),
        }
    }
}

impl Shr for Value {
    type Output = Self;

    fn shr(self, other: Self) -> Self {
        match (self, other) {
            // Signed integers
            (Value::I8(x), Value::I8(y)) => Value::I8(x >> y),
            (Value::I16(x), Value::I16(y)) => Value::I16(x >> y),
            (Value::I32(x), Value::I32(y)) => Value::I32(x >> y),
            (Value::I64(x), Value::I64(y)) => Value::I64(x >> y),
            (Value::I128(x), Value::I128(y)) => Value::I128(x >> y),
            (Value::Isize(x), Value::Isize(y)) => Value::Isize(x >> y),

            // Unsigned integers
            (Value::U8(x), Value::U8(y)) => Value::U8(x >> y),
            (Value::U16(x), Value::U16(y)) => Value::U16(x >> y),
            (Value::U32(x), Value::U32(y)) => Value::U32(x >> y),
            (Value::U64(x), Value::U64(y)) => Value::U64(x >> y),
            (Value::U128(x), Value::U128(y)) => Value::U128(x >> y),
            (Value::Usize(x), Value::Usize(y)) => Value::Usize(x >> y),

            _ => panic!("cannot perform right shift on values of different types"),
        }
    }
}

impl Neg for Value {
    type Output = Self;

    fn neg(self) -> Self {
        match self {
            // Signed integers
            Value::I8(x) => Value::I8(-x),
            Value::I16(x) => Value::I16(-x),
            Value::I32(x) => Value::I32(-x),
            Value::I64(x) => Value::I64(-x),
            Value::I128(x) => Value::I128(-x),
            Value::Isize(x) => Value::Isize(-x),

            // Floats
            Value::F32(x) => Value::F32(-x),
            Value::F64(x) => Value::F64(-x),

            _ => panic!("cannot negate this value type"),
        }
    }
}

impl Not for Value {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Value::Bool(x) => Value::Bool(!x),

            // Bitwise NOT for integers
            Value::I8(x) => Value::I8(!x),
            Value::I16(x) => Value::I16(!x),
            Value::I32(x) => Value::I32(!x),
            Value::I64(x) => Value::I64(!x),
            Value::I128(x) => Value::I128(!x),
            Value::Isize(x) => Value::Isize(!x),

            Value::U8(x) => Value::U8(!x),
            Value::U16(x) => Value::U16(!x),
            Value::U32(x) => Value::U32(!x),
            Value::U64(x) => Value::U64(!x),
            Value::U128(x) => Value::U128(!x),
            Value::Usize(x) => Value::Usize(!x),

            _ => panic!("cannot perform NOT operation on this value type"),
        }
    }
}

impl Value {
    /// Truthy and falsy values
    fn is_truthy(&self) -> bool {
        match self {
            // Numeric types: 0 is falsy, non-zero is truthy
            Value::I8(x) => *x != 0,
            Value::I16(x) => *x != 0,
            Value::I32(x) => *x != 0,
            Value::I64(x) => *x != 0,
            Value::I128(x) => *x != 0,
            Value::Isize(x) => *x != 0,
            Value::U8(x) => *x != 0,
            Value::U16(x) => *x != 0,
            Value::U32(x) => *x != 0,
            Value::U64(x) => *x != 0,
            Value::U128(x) => *x != 0,
            Value::Usize(x) => *x != 0,

            // Floats: 0.0 is falsy, non-zero is truthy (including NaN and infinity)
            Value::F32(x) => *x != 0.0,
            Value::F64(x) => *x != 0.0,

            // Bool: direct value
            Value::Bool(x) => *x,

            // Char: '\0' is falsy, all others are truthy
            Value::Char(c) => *c != '\0',

            // String: empty is falsy, non-empty is truthy
            Value::String(s) => !s.is_empty(),
            Value::Hash(h) => !h.is_empty(), // Assuming Hash has is_empty()

            // Container: empty is falsy, non-empty is truthy
            Value::Container(v) => !v.is_empty(),
        }
    }

    pub fn and(self, other: Self) -> Self {
        let left_truthy = self.is_truthy();
        let right_truthy = other.is_truthy();

        // Return the last evaluated value if both are truthy, or the falsy one
        if left_truthy && right_truthy {
            other
        } else if !left_truthy {
            self
        } else {
            other
        }
    }

    pub fn or(self, other: Self) -> Self {
        let left_truthy = self.is_truthy();
        let right_truthy = other.is_truthy();

        // Return the first truthy value, or the last one if both are falsy
        if left_truthy {
            self
        } else if right_truthy {
            other
        } else {
            other
        }
    }
}

/// Debugging methods
impl Vm {
    /// Run a function given its name, returning the exit code
    /// Mainly used for debugging
    /// TODO: this does not yet handle arguments. Would want this to be called
    /// by a future REPL.
    // Used only for debugging
    fn run_function_by_name(&mut self, name: &str) -> Result<i32> {
        let (_, code_obj) = self.db.get_code_object_by_name(name)?;

        let main = StackFrame {
            code_obj,
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
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use rand::{distr::Alphanumeric, Rng};

    pub fn init_code_obj(code: Bytecode) -> CodeObject {
        CodeObject {
            litpool: vec![Value::int(5), Value::string("hello")],
            argcount: 2, // x and y
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
            labels: Vec::new(),
            localnames: vec!["x".into(), "y".into(), "z".into()],
            code,
        }
    }

    fn init_code_obj_with_pool(code: Bytecode, litpool: Vec<Value>) -> CodeObject {
        CodeObject {
            litpool,
            argcount: 2, // x and y
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
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![
                Instr::LoadLit(0), // 4
                Instr::LoadLit(1), // 3
                Instr::BinOp(BinOp::Add),
                Instr::ReturnVal
            ],
        };

        let hash = vm
            .db
            .insert_code_object_with_name(&func_b, "func_b")
            .unwrap();

        let func_a = CodeObject {
            litpool: vec![Value::I32(10)],
            argcount: 0,
            localnames: vec![],
            labels: Vec::new(),

            code: bytecode![
                Instr::LoadFunc(hash),
                Instr::Call,
                Instr::LoadLit(0),
                Instr::BinOp(BinOp::Mul),
                Instr::ReturnVal
            ],
        };

        vm.db.insert_code_object_with_name(&func_a, "main").unwrap();
        let code = vm.run_main_function().unwrap();
        assert_eq!(code, 70);
    }

    #[test]
    fn test_void_funccall() {
        let mut vm = Vm::new().unwrap();

        let func_b = CodeObject {
            litpool: vec![Value::int(4), Value::int(3)],
            argcount: 0,
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
            localnames: vec![],
            labels: Vec::new(),
            code: bytecode![Instr::LoadFunc(hash), Instr::Call, Instr::Return],
        };
        vm.db.insert_code_object_with_name(&func_a, "main").unwrap();

        let code = vm.run_main_function().unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn test_main_returns_1() {
        let mut vm = Vm::new().unwrap();
        let func = CodeObject {
            litpool: vec![],
            argcount: 0,
            localnames: vec![],
            labels: Vec::new(),
            code: bytecode![Instr::ReturnVal],
        };
        vm.db.insert_code_object_with_name(&func, "main").unwrap();
        assert!(vm.run_main_function().is_err());
    }

    #[test]
    fn test_main_returns_2() {
        let mut vm = Vm::new().unwrap();
        let func = CodeObject {
            litpool: vec![Value::string("break")],
            argcount: 0,
            localnames: vec![],
            labels: Vec::new(),
            code: bytecode![Instr::LoadLit(0), Instr::ReturnVal],
        };
        vm.db.insert_code_object_with_name(&func, "main").unwrap();
        assert!(vm.run_main_function().is_err());
    }

    #[test]
    fn test_main_returns_3() {
        let mut vm = Vm::new().unwrap();
        let func = CodeObject {
            litpool: vec![Value::I32(0)],
            argcount: 0,
            localnames: vec![],
            labels: Vec::new(),
            code: bytecode![Instr::LoadLit(0), Instr::ReturnVal],
        };
        vm.db.insert_code_object_with_name(&func, "main").unwrap();
        assert_eq!(vm.run_main_function().unwrap(), 0);
    }

    #[test]
    fn test_fib() {
        let mut vm = Vm::new().unwrap();
        let fib = CodeObject {
            litpool: vec![Value::I32(0), Value::I32(1), Value::I32(2)],
            argcount: 1,
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
                Instr::ReturnVal,
                // Label 0 (line 18)
                Instr::LoadArg(0), // push n
                Instr::ReturnVal
            ],
        };

        let hash = vm.db.insert_code_object_with_name(&fib, "fib").unwrap();

        let mut f = |n: i32| -> i32 {
            let main = CodeObject {
                litpool: vec![Value::I32(n)],
                argcount: 0,
                localnames: vec![],
                labels: Vec::new(),
                code: bytecode![
                    Instr::LoadLit(0),
                    Instr::LoadFunc(hash),
                    Instr::Call,
                    Instr::ReturnVal
                ],
            };
            vm.db
                .insert_code_object_with_name(&main, &format!("fib_{n}"))
                .unwrap();
            vm.run_function_by_name(&format!("fib_{n}")).unwrap()
        };

        assert_eq!(f(10), 55);
        assert_eq!(f(15), 610);
        assert_eq!(f(25), 75025);
    }
}
