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
    /// Update tasks or records wording/deadlines
    Update(UpdateCommand),
    /// list tasks or records
    #[command(subcommand)]
    List(ListCommand),
}

#[derive(Debug, Args)]
pub struct TaskCommand {
    /// Description of the task
    pub content: String,
    /// Time the task is due, default to EOD
    #[arg(value_parser = validate_timestr)]
    pub timestr: Option<String>,
    /// Category of the task
    #[arg(short, long)]
    pub category: Option<String>,
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
    #[arg(value_parser = validate_index)]
    pub index: usize,
    /// Status, [done|cancelled|remove], default to done.
    #[arg(short, long, value_parser = parse_status, default_value_t = 1)]
    pub status: u8,
}

#[derive(Debug, Args)]
pub struct UpdateCommand {
    /// Index from previous List command
    #[arg(value_parser = validate_index)]
    pub index: usize,
    /// Update target completion time.
    #[arg(short, long, value_parser = validate_timestr)]
    pub target_time: Option<String>,
    /// Update category of the task/record
    #[arg(short, long)]
    pub category: Option<String>,
    /// Update all of content
    #[arg(short='w', long)]
    pub content: Option<String>,
    /// Add to content
    #[arg(short, long)]
    pub add_content: Option<String>,
    /// Edit the status of the tasks
    #[arg(short, long, value_parser = parse_status)]
    pub status: Option<u8>
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
    /// days in the future for tasks to list - mutually exclusive with timestr
    #[arg(short, long)]
    pub days: Option<usize>,
    /// Status to list, default to ongoing tasks
    /// you can filter to [done|cancelled|duplicate] or "all"
    #[arg(short, long, value_parser = parse_status, default_value_t = 0)]
    pub status: u8,
    /// Show overdue tasks - tasks that are scheduled to be completed in the past, but were not
    /// closed. It is assumed that they are already done by default.
    #[arg(short, long, default_value_t = false)]
    pub overdue: bool,
    /// Limit the amount of tasks returned
    #[arg(short, long, default_value_t = 100, value_parser = validate_limit)]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct ListRecordCommand {
    /// Category of the record
    #[arg(short, long)]
    pub category: Option<String>,
    /// days of records to retrieve - e.g. 1 shows record made in the last 24 hours.
    /// value of 7 would show record made in the past week.
    #[arg(short, long)]
    pub days: Option<usize>,
    /// Limit the amount of records returned
    #[arg(short, long, default_value_t = 100, value_parser = validate_limit)]
    pub limit: usize,
}

fn validate_limit(s: &str) -> Result<usize, String> {
    let limit: usize = s.parse().map_err(|_| "Must be a number".to_string())?;
    if limit > 65536 {
        return Err("Limit cannot exceed 65536".to_string());
    }
    Ok(limit)
}

fn validate_index(s: &str) -> Result<usize, String> {
    let index: usize = s.parse().map_err(|_| "Index must be a number".to_string())?;
    if index == 0 {
        return Err("Index must be greater than 0".to_string());
    }
    if index > 65536 {
        return Err("Index cannot exceed 65536".to_string());
    }
    Ok(index)
}

fn validate_timestr(s: &str) -> Result<String, String> {
    match parse_flexible_timestr(s) {
        Ok(_) => Ok(s.to_string()),
        Err(e) => Err(e)
    }
}

fn parse_status(s: &str) -> Result<u8, String> {
    match s.to_lowercase().as_str() {
        "ongoing" => Ok(0), // This is default.
        "done" | "complete" | "completed" => Ok(1),
        "cancelled" | "canceled" | "cancel" => Ok(2),
        "duplicate" => Ok(3),
        "defer" | "suspend" | "shelve" => Ok(4),
        "removed" | "remove" => Ok(5),
        "all" => Ok(255),
        _ => {
            s.parse::<u8>().map_err(|_| 
                format!("Invalid closing code: '{}'. Expected 'completed', 'cancelled', 'duplicate' or a number from 0-255", s)
            )
        }
    }
}
