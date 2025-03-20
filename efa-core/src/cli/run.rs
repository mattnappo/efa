use clap::{Parser, Subcommand};

use efa_core::asm::parser;
use efa_core::db::Database;
use efa_core::solver::resolve_dyn::DynCallResolver;
use efa_core::vm::Vm;

use anyhow::Result;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        input_file: String,
        db_path: Option<String>,
    },
    Dump {
        db_path: String,
    },
}

/// Parse a file, run the DAG solver, hash and insert everything into a
/// code database, and find and run the main function.
fn run_scratch_file(file: &str, db_path: Option<&str>) -> Result<i32> {
    let objs = parser::Parser::parse_file(file)?;

    let resolver = DynCallResolver::new(objs)?;
    let resolved = resolver.resolve_dyn_calls()?;

    let mut vm = if let Some(path) = db_path {
        Vm::persistent(path)?
    } else {
        Vm::new()?
    };

    resolved
        .into_iter()
        .map(|(name, obj)| vm.db.insert_code_object_with_name(&obj, &name))
        .collect::<Result<Vec<_>>>()?;

    let code = vm.run_main_function()?;

    Ok(code)
}

fn dump_db(db_path: &str) -> Result<()> {
    Database::open(db_path)?.dump()
}

fn main() -> Result<()> {
    let args = Args::parse();

    let code = match args.cmd {
        Command::Run {
            input_file,
            db_path,
        } => run_scratch_file(&input_file, db_path.as_deref())
            .expect(&format!("ERROR {}", input_file)),
        Command::Dump { db_path } => {
            dump_db(&db_path)?;
            0
        }
    };

    std::process::exit(code)
}

#[cfg(test)]
mod test {
    use super::*;

    fn dbg_example(file: &str) -> bool {
        match fake_panic(file, run_scratch_file(file, None)) {
            Some(u) => {
                eprintln!("{file} -> {u}");
                false
            }
            None => true,
        }
    }

    fn fake_panic<T>(file: &str, r: Result<T, anyhow::Error>) -> Option<T> {
        match r {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("ERROR {file}: {e}");
                None
            }
        }
    }

    #[test]
    fn test_examples() {
        let failed = std::fs::read_dir("examples/")
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap()
            .into_iter()
            .map(|ex| dbg_example(ex.to_str().unwrap()))
            .any(|x| x);

        if failed {
            panic!();
        }
    }
}
