use std::fmt::Display;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Ok, Result};

use crate::bytecode::{BinOp, Instr, UnaryOp};
use crate::is_valid_name;
use crate::vm::Value;

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
        p.parse().map_err(anyhow::Error::msg)
    }

    pub fn parse(&self) -> Result<Vec<ParseToken>, ParseError> {
        let toks = self
            .contents
            .lines()
            .filter(|l| !l.is_empty())
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
                        let arity: usize = arg[..arg.len() - 2]
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
                    let label = base[..base.len() - 2].to_string();
                    return Result::Ok(ParseToken::Label(label));
                }

                let int_argument = argument
                    .map(|a| a.parse().map_err(|_| ParseError::InvalidArg))
                    .transpose()?;

                let instr = match (base, int_argument) {
                    ("load_arg", Some(arg)) => Instr::LoadArg(arg),
                    ("load_loc", Some(arg)) => Instr::LoadLocal(arg),
                    ("load_lit", Some(arg)) => Instr::LoadLit(arg),
                    ("str_loc", Some(arg)) => Instr::StoreLocal(arg),
                    ("pop", None) => Instr::Pop,

                    // TODO
                    ("load_func", Some(arg)) => {}
                    ("load_dyn", Some(arg)) => {}

                    ("call", None) => {}
                    ("call_self", None) => {}
                    ("ret", None) => {}

                    ("jmp", Some(arg)) => Instr::Jump(arg),
                    ("jmp_t", Some(arg)) => Instr::JumpT(arg),
                    ("jmp_f", Some(arg)) => Instr::JumpF(arg),
                    ("jmp_eq", Some(arg)) => Instr::JumpEq(arg),
                    ("jmp_gt", Some(arg)) => Instr::JumpGt(arg),
                    ("jmp_ge", Some(arg)) => Instr::JumpGe(arg),
                    ("jmp_lt", Some(arg)) => Instr::JumpLt(arg),
                    ("jmp_le", Some(arg)) => Instr::JumpLe(arg),

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

                Result::Ok(ParseToken::Label("x".to_string()))
            })
            .collect::<Vec<Result<ParseToken, ParseError>>>();

        println!("{:?}", toks);

        // todo!()
        Result::Ok(vec![])
    }

    // fn decode_mnemonic(mnemonic) ->
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let parse = Parser::parse_file("./examples/fib.asm").unwrap();
        println!("{:?}", parse);
    }
}
