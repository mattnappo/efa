use clap::Parser;

use efa_core::asm::parser;
use efa_core::solver::resolve_dyn::DynCallResolver;
use efa_core::vm::Vm;

use anyhow::Result;

#[derive(Parser)]
struct Args {
    input_file: String,
    db_path: Option<String>,
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

fn main() -> Result<()> {
    let args = Args::parse();

    let code = run_scratch_file(&args.input_file, args.db_path.as_deref())?;
    std::process::exit(code)
}
