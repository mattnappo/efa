use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Ok, Result};

use crate::bytecode::{BinOp, Instr, UnaryOp};
use crate::vm::Value;
use crate::{is_valid_name, HASH_SIZE};

pub struct Parser {
    file: Option<PathBuf>,
    contents: String,
}

#[derive(Debug)]
enum ParseError {
    UnexpectedArgument,
    ExpectedArgument,
    /// Unknown instruction mnemonic, or bad arguments (missing/present)
    UnknownInstr,
    InvalidArg,
    SyntaxError,
    InvalidIdent,
    InvalidHash,
}

#[derive(Debug)]
enum ParseToken {
    /// Function definition: name, arity
    FuncDef(String, usize),
    /// An instruction
    Instr(Instr),
    /// Label name
    Label(String),
    /// Literal definition
    Lit(Value),
}

impl Parser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Vec<ParseToken>> {
        let contents = fs::read_to_string(&path)?;
        let p = Parser {
            file: Some(path.as_ref().to_path_buf()),
            contents,
        };
        p.parse_function().map_err(anyhow::Error::msg)
    }

    /// Parse the bytecode of a single function
    pub fn parse_function(&self) -> Result<Vec<ParseToken>, ParseError> {
        let code = self.contents.lines().filter(|l| !l.is_empty());

        // Want a map from label names (L0, L1) to label number
        // And an array of offsets (where index is label number)
        let mut j = 0;
        let (label_names, label_offsets): (HashMap<String, usize>, Vec<usize>) = code
            .clone()
            .enumerate()
            .filter_map(|(i, line)| {
                let parts = line.split_whitespace().collect::<Vec<&str>>();
                if parts.len() != 1 {
                    return None;
                }
                let word = parts[0];

                if word.chars().last().unwrap() != ':' {
                    return None;
                }

                let label = &word[0..word.len() - 1];

                // TODO
                // if !is_valid_name(label) {
                //     return Result::Err(ParseError::InvalidIdent);
                // }

                j += 1;
                Some((label.to_string(), i - j))
            })
            .enumerate()
            .map(|(i, (name, offset))| ((name, i), offset))
            .unzip();

        code.map(|l| {
            let parts = l.split_whitespace().collect::<Vec<&str>>();
            dbg!(&parts);
            if parts.len() > 2 {
                return Err(ParseError::UnexpectedArgument);
            }

            let base = parts[0];
            let argument = parts.get(1).copied();

            // If line starts with a $, it is a function
            // Else if line otherwise ends with a colon, it is a label
            // Else, it is an instruction
            let base_first_char = base.chars().nth(0).unwrap(); // todo; remove this unwrap
            match (base_first_char, argument) {
                ('$', Some(arg)) => {
                    let func_name = &base[1..];
                    if arg.chars().last().unwrap() != ':' {
                        return Result::Err(ParseError::SyntaxError);
                    }
                    println!("arg: {arg}");
                    println!("arg-2: {}", &arg[..arg.len() - 1]);
                    let arity: usize = arg[..arg.len() - 1]
                        .parse()
                        .map_err(|_| ParseError::InvalidArg)?;
                    if !is_valid_name(func_name) {
                        return Result::Err(ParseError::InvalidIdent);
                    }
                    return Result::Ok(ParseToken::FuncDef(func_name.to_string(), arity));
                }
                ('$', None) => return Result::Err(ParseError::ExpectedArgument),
                _ => {}
            };

            if argument.is_none() && base.chars().last().unwrap() == ':' {
                let label = base[..base.len() - 1].to_string();
                println!("got label = {label}");
                return Result::Ok(ParseToken::Label(label));
            }

            let int_argument = argument.map(|a| a.parse::<usize>().ok()).flatten();
            let hash_argument = argument.map(|a| hex::decode(a).ok()).flatten();

            let instr = match (base, int_argument) {
                ("load_arg", Some(arg)) => Instr::LoadArg(arg),
                ("load_loc", Some(arg)) => Instr::LoadLocal(arg),
                ("load_lit", Some(arg)) => Instr::LoadLit(arg),
                ("str_loc", Some(arg)) => Instr::StoreLocal(arg),
                ("pop", None) => Instr::Pop,

                // TODO
                ("load_func", None) => {
                    if let Some(hash) = hash_argument {
                        Instr::LoadFunc(
                            hash[..HASH_SIZE]
                                .try_into()
                                .map_err(|_| ParseError::InvalidHash)
                                .unwrap(), // TODO: remove this unwrap
                        )
                    } else {
                        return Err(ParseError::ExpectedArgument);
                    }
                }
                ("load_dyn", None) => {
                    todo!()
                }

                ("call", None) => Instr::Call,
                ("call_self", None) => Instr::CallSelf,
                ("ret", None) => Instr::Return,

                // ("jmp", None) => Instr::Jump(arg),
                // ("jmp_t", None) => Instr::JumpT(arg),
                // ("jmp_f", None) => Instr::JumpF(arg),
                // ("jmp_eq", None) => Instr::JumpEq(arg),
                // ("jmp_gt", None) => Instr::JumpGt(arg),
                // ("jmp_ge", None) => Instr::JumpGe(arg),
                // ("jmp_lt", None) => Instr::JumpLt(arg),
                // ("jmp_le", None) => Instr::JumpLe(arg),
                ("add", None) => Instr::BinOp(BinOp::Add),
                ("mul", None) => Instr::BinOp(BinOp::Mul),
                ("div", None) => Instr::BinOp(BinOp::Div),
                ("sub", None) => Instr::BinOp(BinOp::Sub),
                ("mod", None) => Instr::BinOp(BinOp::Mod),
                ("shl", None) => Instr::BinOp(BinOp::Shl),
                ("shr", None) => Instr::BinOp(BinOp::Shr),
                ("and", None) => Instr::BinOp(BinOp::And),
                ("or", None) => Instr::BinOp(BinOp::Or),
                ("eq", None) => Instr::BinOp(BinOp::Eq),

                ("not", None) => Instr::UnaryOp(UnaryOp::Not),
                ("neg", None) => Instr::UnaryOp(UnaryOp::Neg),

                ("dbg", None) => Instr::Dbg,
                ("nop", None) => Instr::Nop,

                _ => return Err(ParseError::UnknownInstr),
            };

            Result::Ok(ParseToken::Instr(instr))
        })
        .collect::<Result<Vec<ParseToken>, ParseError>>()
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ParseError::UnexpectedArgument => "unexpected argument",
            ParseError::ExpectedArgument => "expected an argument",
            ParseError::UnknownInstr => "unknown instruction or invalid arguments",
            ParseError::InvalidArg => "invalid argument",
            ParseError::SyntaxError => "syntax error",
            ParseError::InvalidIdent => "invalid identifier",
            ParseError::InvalidHash => "invalid hash",
        };
        write!(f, "parser error: {msg}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dbg_f(path: &str) {
        let parse = Parser::parse_file(path).unwrap();
        println!("{:?}", parse);
    }

    #[test]
    fn test_1() {
        // dbg_f("./examples/fib.asm");
        dbg_f("./examples/labels.asm");
    }
}
