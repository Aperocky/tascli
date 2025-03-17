use clap::{Args, Parser, Subcommand};
use crate::args::timestr::parse_flexible_timestr;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct CliArgs {
    #[command(subcommand)]
    pub arguments: Action,
}

#[derive(Debug, Subcommand)]
pub enum Action {
    /// add task with end time
    Task(TaskCommand),
    /// add record
    Record(RecordCommand),
    /// list tasks or records
    List(ListCommand),
    /// Finish task or remove records
    Done(DoneCommand),
}

#[derive(Debug, Args)]
pub struct TaskCommand {
    /// Description of the task
    pub content: String,
    /// Time the task is due, default to EOD
    #[arg(value_parser = validate_timestr)]
    pub timestr: Option<String>,
}

#[derive(Debug, Args)]
pub struct RecordCommand {
    /// Content of the record
    pub content: String,
    /// Category of the record (optional, default to record table)
    #[arg(short, long)]
    pub category: Option<String>,
    /// Time the record should be made, default to current
    #[arg(short = 't', long = "time", value_parser = validate_timestr)]
    pub timestr: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListCommand {
    /// Category to list, e.g. [task, record, record.category]
    pub category: Option<String>,
}

#[derive(Debug, Args)]
pub struct DoneCommand {
    /// Index from previous List command
    pub index: u32,
    /// Closing code, [done|cancelled|remove]
    #[arg(short, long, value_parser = parse_closing_code)]
    pub closing_code: Option<u8>,
}

fn validate_timestr(s: &str) -> Result<String, String> {
    match parse_flexible_timestr(s) {
        Ok(_) => Ok(s.to_string()),
        Err(e) => Err(e)
    }
}

fn parse_closing_code(s: &str) -> Result<u8, String> {
    match s.to_lowercase().as_str() {
        "complete" | "done" | "completed" => Ok(0),
        "cancelled" | "canceled" | "cancel" => Ok(1),
        "duplicate" => Ok(2),
        "removed" | "remove" => Ok(3),
        _ => {
            s.parse::<u8>().map_err(|_| 
                format!("Invalid closing code: '{}'. Expected 'completed', 'cancelled', 'duplicate' or a number from 0-255", s)
            )
        }
    }
}
