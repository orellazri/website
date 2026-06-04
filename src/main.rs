use anyhow::Result;
use clap::Parser;

use crate::{build::build, cli::Cli, serve::serve};

mod build;
mod cli;
mod models;
mod parser;
mod renderer;
mod serve;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        cli::Command::Build => build()?,
        cli::Command::Serve { port } => {
            build()?;
            serve(port)?;
        }
    }

    Ok(())
}
