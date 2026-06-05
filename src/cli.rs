use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ssg")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Build the site into dist/
    Build,
    /// Build and serve the site
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}
