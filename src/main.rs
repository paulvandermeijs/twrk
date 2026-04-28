mod cli;
mod workspace;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    println!("{args:?}");
}
