use anyhow::Result;
use clap::Parser;

mod cli;
mod collect;
mod graph;

use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Graph) => graph::run_graph()?,
        None => collect::run_collect(false)?,
    }
    Ok(())
}
