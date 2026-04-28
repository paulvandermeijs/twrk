mod cli;
mod workspace;
mod path;
mod config;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    println!("{args:?}");
}
