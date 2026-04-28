mod cli;
mod workspace;
mod path;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    println!("{args:?}");
}
