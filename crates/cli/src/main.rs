mod commands;
mod harness;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::TestCommand;

/// Dobo CLI - Data pipeline testing and execution tool
#[derive(Debug, Parser)]
#[command(
    name = "dobo",
    version,
    about = "Data pipeline testing and execution tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Execute test scenarios
    Test(TestCommand),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Test(cmd) => cmd.execute()?,
    };

    std::process::exit(exit_code);
}
