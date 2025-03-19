mod args;
mod db;

use args::parser::CliArgs;
use clap::Parser;

fn main() {
    let cli_args = CliArgs::parse();
    match db::conn::connect() {
        Ok(_) => {
            println!("db established successfully")
        }
        Err(e) => {
            println!("Sqlite error {}", e)
        }
    }

    println!("{:?}", cli_args);
}
