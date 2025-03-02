use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Ok, Result};

use crate::bytecode::{BinOp, Bytecode, Instr, UnaryOp};
use crate::vm::{CodeObject, Value};
use crate::{is_valid_name, HASH_SIZE};

pub struct Parser;

#[derive(Debug)]
struct PartialParse {
    tokens: Vec<ParseToken>,
    labels: Vec<usize>,
    is_void: bool,
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
    UnknownLabel,
    NoFunctionDef,
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

#[derive(Debug)]
struct Parse {
    func_name: String,
    code_obj: CodeObject,
}

impl Parser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Vec<Parse>> {
        let contents = fs::read_to_string(&path)?;
        let functions = Self::split_functions(&contents).map_err(anyhow::Error::msg)?;
        functions
            .into_iter()
            .map(|func| {
                Self::parse_function(&func)
                    .and_then(Self::resolve_parse)
                    .map_err(anyhow::Error::msg)
            })
            .collect::<Result<Vec<Parse>>>()
    }

    fn is_func_def(line: &str) -> bool {
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        if parts.len() != 2 {
            return false;
        }

        let name = parts[0];
        let arg = parts[1];

        let d = name.chars().nth(0).unwrap();
        let c = arg.chars().last().unwrap();

        if d == '$' && c == ':' {
            // Now it should be a function def line
            let name = &name[1..];
            let arity = &arg[..arg.len() - 1];
            return is_valid_name(name) && arity.parse::<usize>().is_ok();
        }
        false
    }

    fn split_functions(contents: &str) -> Result<Vec<String>> {
        Ok(contents
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.trim())
            .fold(vec![], |mut acc, line| {
                if Self::is_func_def(line) {
                    acc.push(vec![line]);
                } else if let Some(last) = acc.last_mut() {
                    last.push(line);
                } else {
                    acc.push(vec![line])
                }
                acc
            })
            .into_iter()
            .map(|func| func.join("\n"))
            .collect())
    }

    /// Parse the bytecode of a single function
    pub fn parse_function(function: &str) -> Result<PartialParse, ParseError> {
        let code = function.lines();

        // Want a map from label names (L0, L1) to label number
        // And an array of offsets (where index is label number)
        let mut j = 0;
        // TODO: factor out into own method
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

        let mut is_void: bool = false;

        let tokens = code
            .map(|l| {
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
                    return Result::Ok(ParseToken::Label(label));
                }

                let int_argument = argument.map(|a| a.parse::<usize>().ok()).flatten();
                let hash_argument = argument.map(|a| hex::decode(a).ok()).flatten(); // Remove the 0x
                let str_argument = if int_argument.is_some() {
                    None
                } else {
                    argument
                };

                // dbg!(&l);
                // dbg!(&argument);
                // dbg!(&int_argument);
                // dbg!(&hash_argument);
                // dbg!(&str_argument);

                let instr = match (base, int_argument, str_argument) {
                    ("load_arg", Some(arg), None) => Instr::LoadArg(arg),
                    ("load_loc", Some(arg), None) => Instr::LoadLocal(arg),
                    ("load_lit", Some(arg), None) => Instr::LoadLit(arg),
                    ("str_loc", Some(arg), None) => Instr::StoreLocal(arg),
                    ("pop", None, None) => Instr::Pop,

                    // TODO
                    ("load_func", None, None) => {
                        if let Some(hash) = hash_argument {
                            Instr::LoadFunc(
                                hash[..HASH_SIZE]
                                    .try_into()
                                    .map_err(|_| ParseError::InvalidHash)?,
                            )
                        } else {
                            return Err(ParseError::ExpectedArgument);
                        }
                    }
                    ("load_dyn", None, Some(arg)) => {
                        let func_name = &arg[1..];
                        Instr::LoadDyn(func_name.to_string())
                    }

                    (op, None, Some(arg)) if op.starts_with("jmp") => {
                        Self::get_jump_instr(op, &label_names, arg)?
                    }

                    ("call", None, None) => Instr::Call,
                    ("call_self", None, None) => Instr::CallSelf,
                    ("ret", None, None) => Instr::Return,
                    ("ret_val", None, None) => {
                        is_void = false;
                        Instr::Return
                    }

                    ("add", None, None) => Instr::BinOp(BinOp::Add),
                    ("mul", None, None) => Instr::BinOp(BinOp::Mul),
                    ("div", None, None) => Instr::BinOp(BinOp::Div),
                    ("sub", None, None) => Instr::BinOp(BinOp::Sub),
                    ("mod", None, None) => Instr::BinOp(BinOp::Mod),
                    ("shl", None, None) => Instr::BinOp(BinOp::Shl),
                    ("shr", None, None) => Instr::BinOp(BinOp::Shr),
                    ("and", None, None) => Instr::BinOp(BinOp::And),
                    ("or", None, None) => Instr::BinOp(BinOp::Or),
                    ("eq", None, None) => Instr::BinOp(BinOp::Eq),

                    ("not", None, None) => Instr::UnaryOp(UnaryOp::Not),
                    ("neg", None, None) => Instr::UnaryOp(UnaryOp::Neg),
                    _ => return Err(ParseError::UnknownInstr),
                };

                Result::Ok(ParseToken::Instr(instr))
            })
            .collect::<Result<Vec<ParseToken>, ParseError>>()?;

        Result::Ok(PartialParse {
            tokens,
            labels: label_offsets,
            is_void,
        })
    }

    fn get_jump_instr(
        op: &str,
        label_names: &HashMap<String, usize>,
        arg: &str,
    ) -> Result<Instr, ParseError> {
        dbg!(&label_names);
        let label_idx = label_names.get(arg).ok_or(ParseError::UnknownLabel)?;
        match op {
            "jmp" => Result::Ok(Instr::Jump(*label_idx)),
            "jmp_t" => Result::Ok(Instr::JumpT(*label_idx)),
            "jmp_f" => Result::Ok(Instr::JumpF(*label_idx)),
            "jmp_eq" => Result::Ok(Instr::JumpEq(*label_idx)),
            "jmp_gt" => Result::Ok(Instr::JumpGt(*label_idx)),
            "jmp_ge" => Result::Ok(Instr::JumpGe(*label_idx)),
            "jmp_lt" => Result::Ok(Instr::JumpLt(*label_idx)),
            "jmp_le" => Result::Ok(Instr::JumpLe(*label_idx)),
            _ => Err(ParseError::UnknownInstr),
        }
    }

    fn resolve_parse(partial: PartialParse) -> Result<Parse, ParseError> {
        let (name, argcount) = partial
            .tokens
            .iter()
            .find_map(|tok| {
                if let ParseToken::FuncDef(name, arity) = tok {
                    Some((name, *arity))
                } else {
                    None
                }
            })
            .ok_or(ParseError::NoFunctionDef)?;

        let code = partial
            .tokens
            .iter()
            .filter_map(|token| match token {
                ParseToken::Instr(instr) => Some(instr.clone()),
                _ => None,
            })
            .collect();

        Result::Ok(Parse {
            func_name: name.to_owned(),
            code_obj: CodeObject {
                litpool: Vec::new(), // TODO
                argcount,
                is_void: partial.is_void,
                localnames: Vec::new(),
                labels: partial.labels,
                code: Bytecode::new(code),
            },
        })
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
            ParseError::UnknownLabel => "reference to undefined label",
            ParseError::NoFunctionDef => "no function definition",
        };
        write!(f, "parser error: {msg}")
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn dbg_f(path: &str) {
        let parse = Parser::parse_file(path).unwrap();
        println!("{:#?}", parse);
    }

    #[test]
    fn test_1() {
        dbg_f("./examples/fib.asm");
        // dbg_f("./examples/labels.asm");
    }

    #[test]
    fn test_is_funcdef() {
        assert!(Parser::is_func_def("$fib 3:"));
        assert!(Parser::is_func_def("$fibb 33:"));
        assert!(!Parser::is_func_def("$fibb 33"));
        assert!(!Parser::is_func_def("fibb 99:"));
    }
}
