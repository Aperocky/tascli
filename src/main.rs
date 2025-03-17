mod args;

use args::parser::CliArgs;
use clap::Parser;

fn main() {
    let cli_args = CliArgs::parse();

    println!("{:?}", cli_args);
}
