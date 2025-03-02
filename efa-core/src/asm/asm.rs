use clap::Parser;
use efa_core::asm::parser;
use efa_core::vm;
use std::error::Error as StdError;

#[derive(Parser)]
struct Args {
    input_file: String,
    db_path: Option<String>,
}

// TODO: need to make call DAG

fn main() -> Result<(), Box<dyn StdError>> {
    let args = Args::parse();
    let objs = parser::Parser::parse_file(&args.input_file)?;

    /* test code for fib example only */

    let fib = &objs[0];
    let main = &objs[1];

    let mut vm = if let Some(path) = args.db_path {
        vm::Vm::initialize(path)
    } else {
        vm::Vm::new()
    }?;

    vm.db
        .insert_code_object_with_name(&fib.code_obj, &fib.func_name)?;

    let code = vm.run_main_function(&main.code_obj)?;

    println!("ret = {}", code);

    std::process::exit(code)
}
