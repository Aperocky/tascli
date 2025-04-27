mod actions;
mod args;
mod config;
mod db;

use std::process::exit;

use actions::display::print_red;
use args::parser::CliArgs;
use clap::Parser;

fn main() {
    let cli_args = CliArgs::parse();
    let conn = match db::conn::connect() {
        Ok(conn) => conn,
        Err(err) => {
            print_red(&format!("Error connecting to db file: {}", err));
            exit(1)
        }
    };
    let result = actions::handler::handle_commands(&conn, cli_args);
    if result.is_err() {
        print_red(&format!("Error: {}", result.unwrap_err()));
        exit(1)
    }
}

#[cfg(test)]
pub mod tests;
