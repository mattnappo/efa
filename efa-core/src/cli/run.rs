use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};

use efa_core::cli::commands as cli;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
// #[command(version, about, long_about = None)]
enum Command {
    /// Run a bytecode assembly file
    Run {
        input_file: String,
        db_path: Option<String>,
    },

    /// Disassemble a code database
    Dis { db_path: String },

    /// Roundtrip a bytecode assembly file
    Rt {
        input_file: String,

        /// Run the file
        #[clap(long, short, action=ArgAction::SetFalse)]
        run: bool,
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
        Command::Rt { input_file, run } => {
            cli::roundtrip_file(&input_file, run)?;
            0
        }
    };

    std::process::exit(code)
}
