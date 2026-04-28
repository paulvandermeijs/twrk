mod cli;
mod workspace;
mod path;
mod config;
mod session;
mod git;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    println!("{args:?}");
}
