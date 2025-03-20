use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::path::Path;

use anyhow::{Ok, Result};
use regex::Regex;

use crate::bytecode::{BinOp, Bytecode, Instr, UnaryOp};
use crate::hash_from_str;
use crate::is_valid_name;
use crate::vm::{CodeObject, Value};

pub struct Parser;

#[derive(Debug)]
struct PartialParse {
    tokens: Vec<ParseToken>,
    labels: Vec<usize>,
    num_locals: usize,
    is_void: bool,
    literals: Vec<Value>,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedArgument,
    ExpectedArgument,
    InvalidArg,
    SyntaxError,

    InvalidIdent(String),
    InvalidLabelName(String),
    InvalidHash,
    InvalidStrLit,
    InvalidFuncDef,
    InvalidLiteral,

    /// Unknown instruction mnemonic, or bad arguments (missing/present)
    UnknownInstr(String),
    UnknownLabel,

    NoFunctionDef,
    RegexError(String),

    Error(anyhow::Error),
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
pub struct Parse {
    pub func_name: String,
    pub code_obj: CodeObject,
}

impl Parser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Vec<Parse>> {
        let contents = fs::read_to_string(&path)?;
        let contents = Self::preprocess(&contents);
        let functions = Self::split_functions(&contents).map_err(anyhow::Error::msg)?;
        functions
            .into_iter()
            .map(|func| {
                Self::parse_function(&func)
                    .and_then(Self::finalize_parse)
                    .map_err(anyhow::Error::msg)
            })
            .collect::<Result<Vec<Parse>>>()
    }

    fn is_func_def(line: &str) -> Option<Result<(String, usize), ParseError>> {
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        if parts.len() != 2 {
            return None;
        }

        let name = parts[0];
        let arg = parts[1];

        let d = name.chars().nth(0).unwrap();
        let c = arg.chars().last().unwrap();

        if d == '$' && c == ':' {
            // Now it should be a function def line
            let name = &name[1..];
            let arity = &arg[..arg.len() - 1];
            let parsed = arity.parse::<usize>();
            if is_valid_name(name) && parsed.is_ok() {
                return Some(Result::Ok((name.to_string(), parsed.unwrap())));
            } else {
                return Some(Err(ParseError::InvalidFuncDef));
            }
        }
        None
    }

    fn split_functions(file_contents: &str) -> Result<Vec<String>> {
        Ok(file_contents
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.trim())
            .fold(vec![], |mut acc, line| {
                if Self::is_func_def(line).is_some() {
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

    fn get_labels(
        function: &str,
    ) -> Result<(HashMap<String, usize>, Vec<usize>), ParseError> {
        // Want a map from label names (L0, L1) to label number
        // And an array of offsets (where index is label number)
        let mut j = 0;

        let result = function
            .lines()
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
                if !is_valid_name(label) {
                    return Some(Result::Err(ParseError::InvalidLabelName(
                        label.to_string(),
                    )));
                }
                j += 1;
                Some(Result::Ok((label.to_string(), i - j)))
            })
            .collect::<Result<Vec<(String, usize)>, ParseError>>()?;

        let (label_names, label_offsets) = result
            .into_iter()
            .enumerate()
            .map(|(i, (name, offset))| ((name, i), offset))
            .unzip();

        Result::Ok((label_names, label_offsets))
    }

    fn get_literals(function: &str) -> Result<Vec<Value>, ParseError> {
        let code = function.lines();

        code.filter(|line| !line.is_empty())
            .filter(|line| line.chars().nth(0).unwrap() == '.')
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return Some(Err(ParseError::ExpectedArgument));
                }

                let first = parts[0];
                let arg = parts[1];
                let arg_chars: Vec<char> = arg.chars().collect();

                let opcode = &first[1..];
                if opcode != "lit" {
                    return Some(Err(ParseError::InvalidLiteral));
                }

                // Bool case
                if arg == "true" {
                    return Some(Result::Ok(Value::Bool(true)));
                }
                if arg == "false" {
                    return Some(Result::Ok(Value::Bool(false)));
                }

                // String case
                if arg_chars[0] == '"' {
                    let s = Self::get_str_lit(line).map(|s| Value::String(s));
                    return Some(s);
                }

                // Hash case
                if arg.len() >= 2 && arg.starts_with("0x") {
                    let h = hash_from_str(arg).map(|bytes| Value::Hash(bytes));
                    return Some(h.map_err(ParseError::Error));
                }

                // Int case
                if let Result::Ok(int) = arg.parse::<i32>() {
                    return Some(Result::Ok(Value::I32(int)));
                }

                None
            })
            .collect::<Result<Vec<Value>, ParseError>>()
    }

    fn get_num_locals(tokens: &[ParseToken]) -> Result<usize, ParseError> {
        let num = tokens
            .into_iter()
            .filter_map(|token| match token {
                ParseToken::Instr(Instr::LoadLocal(i))
                | ParseToken::Instr(Instr::StoreLocal(i)) => Some(i + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);

        Result::Ok(num)
    }

    fn get_str_lit(line: &str) -> Result<String, ParseError> {
        let pattern = r#"\.lit\s*\"([^\"]*)\""#;
        let re =
            Regex::new(pattern).map_err(|e| ParseError::RegexError(e.to_string()))?;
        let matches: Vec<String> = re
            .captures_iter(line)
            .filter_map(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .collect();

        if matches.len() == 1 {
            Result::Ok(matches[0].clone())
        } else {
            Err(ParseError::InvalidStrLit)
        }
    }

    /// Parse the bytecode of a single function
    fn parse_function(function: &str) -> Result<PartialParse, ParseError> {
        let literals = Self::get_literals(function)?;
        let code = function
            .lines()
            .filter(|line| !line.contains("."))
            .collect::<Vec<&str>>()
            .join("\n");
        let (label_names, label_offsets) = Self::get_labels(&code)?;
        let code = code.lines();

        let mut is_void: bool = true;
        let tokens = code
            .map(|line| {
                let parts = line.split_whitespace().collect::<Vec<&str>>();
                if parts.len() > 2 {
                    return Err(ParseError::UnexpectedArgument);
                }

                let base = parts[0];
                let argument = parts.get(1);

                // Line is a function definition, or an incorrect function definition
                match Self::is_func_def(line) {
                    Some(Result::Ok((name, arity))) => {
                        return Result::Ok(ParseToken::FuncDef(name, arity));
                    }
                    Some(Err(e)) => return Err(e),
                    None => {}
                };

                // Line is a label
                if argument.is_none() && base.chars().last().unwrap() == ':' {
                    let label = base[..base.len() - 1].to_string();
                    return Result::Ok(ParseToken::Label(label));
                }

                // Line is an instruction

                // Setup arguments
                let int_argument = argument.map(|a| a.parse::<usize>().ok()).flatten();
                let str_argument = if int_argument.is_some() {
                    None
                } else {
                    argument
                };

                // dbg!(&line);
                // dbg!(&argument);
                // dbg!(&int_argument);
                // dbg!(&str_argument);

                // Decode instruction
                let instr = match (base, int_argument, str_argument) {
                    ("load_arg", Some(arg), None) => Instr::LoadArg(arg),
                    ("load_loc", Some(arg), None) => Instr::LoadLocal(arg),
                    ("load_lit", Some(arg), None) => Instr::LoadLit(arg),
                    ("store_loc", Some(arg), None) => Instr::StoreLocal(arg),
                    ("pop", None, None) => Instr::Pop,
                    ("dup", None, None) => Instr::Dup,

                    // TODO
                    ("load_func", None, Some(hash)) => {
                        Instr::LoadFunc(hash_from_str(hash).map_err(ParseError::Error)?)
                    }
                    ("load_func", None, None) => {
                        return Err(ParseError::ExpectedArgument);
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

                    ("nop", None, None) => Instr::Nop,
                    ("dbg", None, None) => Instr::Dbg,
                    _ => return Err(ParseError::UnknownInstr(line.to_string())),
                };

                Result::Ok(ParseToken::Instr(instr))
            })
            .collect::<Result<Vec<ParseToken>, ParseError>>()?;

        let num_locals = Self::get_num_locals(&tokens)?;

        Result::Ok(PartialParse {
            tokens,
            labels: label_offsets,
            num_locals,
            is_void,
            literals,
        })
    }

    fn get_jump_instr(
        op: &str,
        label_names: &HashMap<String, usize>,
        arg: &str,
    ) -> Result<Instr, ParseError> {
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
            _ => Err(ParseError::UnknownInstr(op.to_string())),
        }
    }

    // TODO: add imports like #include in C
    fn preprocess(contents: &str) -> String {
        // Remove the comments
        contents
            .lines()
            .map(|line| {
                let mut inside_string = false;
                let mut result = String::new();
                let mut chars = line.chars().peekable();

                // Special care taken here to allow .lit "#not a comment"
                while let Some(c) = chars.next() {
                    if c == '"' || c == '\'' {
                        inside_string = !inside_string;
                        result.push(c);
                    } else if c == '#' && !inside_string {
                        break;
                    } else {
                        result.push(c);
                    }
                }

                result.trim().to_string()
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn finalize_parse(partial: PartialParse) -> Result<Parse, ParseError> {
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

        let localnames = (0..argcount + partial.num_locals)
            .collect::<Vec<usize>>()
            .into_iter()
            .map(|t| format!("x{t}"))
            .collect();

        Result::Ok(Parse {
            func_name: name.to_owned(),
            code_obj: CodeObject {
                litpool: partial.literals,
                argcount,
                is_void: partial.is_void,
                localnames,
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
            ParseError::UnknownInstr(instr) => {
                &format!("unknown instruction or invalid arguments: '{instr}'")
            }
            ParseError::InvalidArg => "invalid argument",
            ParseError::SyntaxError => "syntax error",
            ParseError::InvalidIdent(s) => &format!("invalid identifier '{s}'"),
            ParseError::InvalidLabelName(s) => &format!("invalid label name '{s}'"),
            ParseError::InvalidHash => "invalid hash",
            ParseError::UnknownLabel => "reference to undefined label",
            ParseError::NoFunctionDef => "no function definition",
            ParseError::InvalidFuncDef => "invalid function definition",
            ParseError::InvalidLiteral => "invalid literal definition",
            ParseError::InvalidStrLit => "invalid string literal",
            ParseError::RegexError(e) => &format!("regex: {e}"),
            ParseError::Error(e) => &format!("{e}"),
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
    fn test_examples() {
        dbg_f("./examples/fib.asm");
        dbg_f("./examples/lits.asm");
        dbg_f("./examples/double.asm");
        dbg_f("./examples/call.asm");
        dbg_f("./examples/comments.asm");
    }

    #[test]
    fn test_is_funcdef() {
        assert!(matches!(
            Parser::is_func_def("$fib 3:"),
            Some(Result::Ok(_))
        ));
        assert!(matches!(
            Parser::is_func_def("$fibb 33:"),
            Some(Result::Ok(_))
        ));
        assert!(matches!(Parser::is_func_def("$fibb 33"), None));
        assert!(matches!(Parser::is_func_def("fibb 99:"), None));
    }

    #[test]
    fn test_num_locals() {
        assert_eq!(
            Parser::get_num_locals(&[
                ParseToken::Instr(Instr::LoadLocal(0)),
                ParseToken::Instr(Instr::StoreLocal(1)),
            ])
            .unwrap(),
            2
        );

        assert_eq!(
            Parser::get_num_locals(&[ParseToken::Instr(Instr::Nop)]).unwrap(),
            0
        );

        assert_eq!(
            Parser::get_num_locals(&[ParseToken::Instr(Instr::LoadLocal(0))]).unwrap(),
            1
        );
    }
}
