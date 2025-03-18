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
    /// Finish task or remove records
    Done(DoneCommand),
    /// list tasks or records
    #[command(subcommand)]
    List(ListCommand),
}

#[derive(Debug, Args)]
pub struct TaskCommand {
    /// Description of the task
    pub content: String,
    /// Category of the task
    #[arg(short, long)]
    pub category: Option<String>,
    /// Time the task is due, default to EOD
    #[arg(value_parser = validate_timestr)]
    pub timestr: Option<String>,
}

#[derive(Debug, Args)]
pub struct RecordCommand {
    /// Content of the record
    pub content: String,
    /// Category of the record
    #[arg(short, long)]
    pub category: Option<String>,
    /// Time the record should be made, default to current
    #[arg(short = 't', long = "time", value_parser = validate_timestr)]
    pub timestr: Option<String>,
}

#[derive(Debug, Args)]
pub struct DoneCommand {
    /// Index from previous List command
    pub index: u32,
    /// Closing code, [done|cancelled|remove]
    #[arg(short, long, value_parser = parse_closing_code)]
    pub closing_code: Option<u8>,
}

#[derive(Debug, Subcommand)]
pub enum ListCommand {
    /// List tasks
    Task(ListTaskCommand),
    /// List records
    Record(ListRecordCommand),
}

#[derive(Debug, Args)]
pub struct ListTaskCommand {
    /// Target completion date - e.g. "today"", only list task marked to be completed today.
    pub timestr: Option<String>,
    /// Category of the task
    #[arg(short, long)]
    pub category: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListRecordCommand {
    /// Category of the record
    #[arg(short, long)]
    pub category: Option<String>,
    /// days of records to retrieve - e.g. 1 shows record made in the last 24 hours
    #[arg(short, long)]
    pub days: Option<u32>
}

fn validate_timestr(s: &str) -> Result<String, String> {
    match parse_flexible_timestr(s) {
        Ok(_) => Ok(s.to_string()),
        Err(e) => Err(e)
    }
}

fn parse_closing_code(s: &str) -> Result<u8, String> {
    match s.to_lowercase().as_str() {
        "ongoing" => Ok(0), // This is default.
        "complete" | "done" | "completed" => Ok(1),
        "cancelled" | "canceled" | "cancel" => Ok(2),
        "duplicate" => Ok(3),
        "removed" | "remove" => Ok(4),
        _ => {
            s.parse::<u8>().map_err(|_| 
                format!("Invalid closing code: '{}'. Expected 'completed', 'cancelled', 'duplicate' or a number from 0-255", s)
            )
        }
    }
}
