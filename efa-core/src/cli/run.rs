use anyhow::Result;
use clap::{Parser, Subcommand};

use efa_core::cli::commands as cli;

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
    Dis {
        db_path: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    let code = match args.cmd {
        Command::Run {
            input_file,
            db_path,
        } => cli::run_scratch_file(&input_file, db_path.as_deref())
            .expect(&format!("ERROR {}", input_file)),
        Command::Dis { db_path } => {
            cli::disassemble_db(&db_path)?;
            0
        }
    };

    std::process::exit(code)
}
