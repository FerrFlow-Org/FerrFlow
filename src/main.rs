mod cli;
mod config;
mod git;
mod conventional_commits;
mod versioning;
mod changelog;
mod monorepo;
mod formats;
mod release;

use anyhow::Result;
use cli::Cli;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}
