use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "winbox-stats", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Render PNG graphs directly from all *.sqlite files in the current directory
    Graph,
}
