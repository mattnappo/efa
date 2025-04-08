use std::fs;
use std::io::prelude::*;

use anyhow::Result;

use crate::asm::parser;
use crate::db::Database;
use crate::solver::resolve_dyn::DynCallResolver;
use crate::vm::Vm;

/// Run a bytecode assembly file.
/// Parse a file, run the DAG solver, hash and insert everything into a
/// code database, and find and run the main function.
pub fn run_scratch_file(file: &str, db_path: Option<&str>) -> Result<i32> {
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

pub fn disassemble_db(db_path: &str) -> Result<String> {
    let dis = Database::open(db_path)?.disassemble()?;
    print!("{dis}");
    Ok(dis)
}

// TODO: support run flag
pub fn roundtrip_file(file: &str, _run: bool) -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let db_file = tmp.path().join("test.db").display().to_string();
    let dis_file = tmp.path().join("dis.asm").display().to_string();

    // Run the original file
    let ret_val = run_scratch_file(file, Some(&db_file))?;

    // Disassemble the db and write the disassembled contents to a file
    let dis = disassemble_db(&db_file)?;
    let mut f = fs::File::create(&dis_file)?;
    f.write_all(dis.as_bytes())?;

    // Run the dis file
    let ret_val_dis = run_scratch_file(&dis_file, None)?;
    assert_eq!(ret_val, ret_val_dis);

    Ok(())
}

#[cfg(test)]
mod integration_test {
    //! These serve as integration tests, essentially

    use super::*;

    macro_rules! run {
        ($file:expr) => {
            run_scratch_file($file, None).expect(&format!("ERROR {}", $file))
        };
    }

    #[test]
    fn test_examples() {
        assert_eq!(run!("examples/args.asm"), 6);
        assert_eq!(run!("examples/compound_if.asm"), 0);
        assert_eq!(run!("examples/call.asm"), 7);
        assert_eq!(run!("examples/double.asm"), 0);
        assert_eq!(run!("examples/fib.asm"), 6765);
        assert_eq!(run!("examples/lits.asm"), 44);
        assert_eq!(run!("examples/sum_squares.asm"), 55);
        assert_eq!(run!("examples/primes.asm"), 97);
        assert_eq!(run!("examples/isqrt.asm"), 225);
        assert_eq!(run!("examples/main.asm"), 1);
        assert_eq!(run!("examples/array_2d.asm"), 6);
        assert_eq!(run!("examples/array_map.asm"), 90);
    }

    #[test]
    fn test_roundtrips() {
        std::fs::read_dir("examples/")
            .unwrap()
            .map(|res| res.map(|e| e.path().display().to_string()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap()
            .into_iter()
            .try_for_each(|ref f| roundtrip_file(f, true))
            .unwrap();
    }
}
