mod cli;
mod workspace;
mod path;
mod config;
mod session;
mod git;
mod tmux;
mod run;
mod picker;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    println!("{args:?}");
}
