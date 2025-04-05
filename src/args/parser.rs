use clap::{
    Args,
    Parser,
    Subcommand,
};
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
    /// Finish tasks
    Done(DoneCommand),
    /// Update tasks or records wording/deadlines
    Update(UpdateCommand),
    /// Delete Records or Tasks
    Delete(DeleteCommand),
    /// list tasks or records
    #[command(subcommand)]
    List(ListCommand),
}

#[derive(Debug, Args)]
pub struct TaskCommand {
    /// Description of the task
    #[arg(value_parser = |s: &str| syntax_helper("task", s))]
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
    #[arg(value_parser = |s: &str| syntax_helper("record", s))]
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
pub struct DeleteCommand {
    /// Index from previous List command
    #[arg(value_parser = validate_index)]
    pub index: usize,
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
    /// accept ongoing|done|cancelled|duplicate|suspended|pending
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
    #[arg(short, long, conflicts_with = "timestr")]
    pub days: Option<usize>,
    /// Status to list, default to "open"
    /// you can filter individually to ongoing|done|cancelled|duplicate|suspended|pending
    /// or aggregate status like open|closed|all
    #[arg(short, long, value_parser = parse_status, default_value_t = 254)]
    pub status: u8,
    /// Show overdue tasks - tasks that are scheduled to be completed in the past
    /// but were not closed, these tasks are not returned by default
    #[arg(short, long, default_value_t = false)]
    pub overdue: bool,
    /// Limit the amount of tasks returned
    #[arg(short, long, default_value_t = 100, value_parser = validate_limit)]
    pub limit: usize,
    /// Next page if the previous list command reached limit
    #[arg(short, long, default_value_t = false)]
    pub next_page: bool
}

#[derive(Debug, Args)]
pub struct ListRecordCommand {
    /// Category of the record
    #[arg(short, long)]
    pub category: Option<String>,
    /// days of records to retrieve - e.g. 1 shows record made in the last 24 hours.
    /// value of 7 would show record made in the past week.
    #[arg(short, long, conflicts_with_all = ["starting_date", "ending_date"])]
    pub days: Option<usize>,
    /// Limit the amount of records returned
    #[arg(short, long, default_value_t = 100, value_parser = validate_limit)]
    pub limit: usize,
    /// List the record starting from this time
    /// If this is date only, then it is non-inclusive
    #[arg(short, long, value_parser = validate_timestr, conflicts_with = "days")]
    pub starting_time: Option<String>,
    /// List the record ending at this time
    /// If this is date only, then it is inclusive
    #[arg(short, long, value_parser = validate_timestr, conflicts_with = "days")]
    pub ending_time: Option<String>,
    /// Next page if the previous list command reached limit
    #[arg(short, long, default_value_t = false)]
    pub next_page: bool
}

fn syntax_helper(cmd: &str, s: &str) -> Result<String, String> {
    if s == "list" {
        return Err(format!("Do you mean 'list {}' instead of '{} list'", cmd, cmd));
    }
    if s == "help" {
        return Err("Do you mean --help instead of help".to_string());
    }
    Ok(s.to_string())
}

fn validate_limit(s: &str) -> Result<usize, String> {
    let limit: usize = s.parse().map_err(|_| "Must be a number".to_string())?;
    if limit < 1 {
        return Err("Limit cannot be less than 1".to_string());
    }
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
        "deferred" | "suspended" | "shelved" => Ok(4),
        "removed" | "remove" => Ok(5),
        "pending" => Ok(6),
        "closed" => Ok(253), // combination of done | cancelled | duplicate | removed
        "open" => Ok(254), // combination of ongoing | pending | suspended
        "all" => Ok(255), // all status
        _ => {
            s.parse::<u8>().map_err(|_| 
                format!("Invalid closing code: '{}'. Expected 'completed', 'cancelled', 'duplicate' or a number from 0-255", s)
            )
        }
    }
}
