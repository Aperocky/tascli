mod actions;
mod args;
mod db;

use args::parser::CliArgs;
use clap::Parser;

fn main() {
    let cli_args = CliArgs::parse();
    let conn = db::conn::connect().unwrap();
    let result = actions::handler::handle_commands(&conn, cli_args);
    if result.is_err() {
        println!("Error executing tascli: {:?}", result.unwrap_err());
    }
}

#[cfg(test)]
pub mod tests;
