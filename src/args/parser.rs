use clap::{
    Args,
    Parser,
    Subcommand,
};
use crate::args::timestr::{parse_flexible_timestr, parse_recurring_timestr};

/// a simple CLI tool for tracking tasks and records from terminal
///
/// data is stored at ~/.local/share/tascli/tascli.db,
/// or where defined in config at ~/.config/tascli/config.json
#[derive(Debug, Parser)]
#[command(author, version)]
pub struct CliArgs {
    #[command(subcommand)]
    pub arguments: Action,
}

#[derive(Debug, Subcommand)]
pub enum Action {
    /// add task
    Task(TaskCommand),
    /// add record
    Record(RecordCommand),
    /// complete task and generates a corresponding record entry
    Done(DoneCommand),
    /// update task and record entries
    Update(UpdateCommand),
    /// delete task or record
    Delete(DeleteCommand),
    /// list tasks or records
    #[command(subcommand)]
    List(ListCommand),
    /// operations and statistics
    #[command(subcommand)]
    Ops(OpsCommand),
}

#[derive(Debug, Args)]
pub struct TaskCommand {
    /// description of the task
    #[arg(value_parser = |s: &str| syntax_helper("task", s))]
    pub content: String,
    /// time the task is due for completion, default to EOD,
    /// If it is a schedule, then a recurring task would be created.
    #[arg(value_parser = validate_timestr)]
    pub timestr: Option<String>,
    /// category of the task
    #[arg(short, long)]
    pub category: Option<String>,
}

#[derive(Debug, Args)]
pub struct RecordCommand {
    /// content of the record
    #[arg(value_parser = |s: &str| syntax_helper("record", s))]
    pub content: String,
    /// category of the record
    #[arg(short, long)]
    pub category: Option<String>,
    /// time the record is made,
    /// default to current time
    #[arg(short = 't', long = "time", value_parser = validate_timestr)]
    pub timestr: Option<String>,
}

#[derive(Debug, Args)]
pub struct DoneCommand {
    /// index from previous list command, or timestr (e.g. 'today') for interactive completion
    pub target: String,
    /// optional status, default to done.
    #[arg(short, long, value_parser = parse_status, default_value_t = 1)]
    pub status: u8,
    /// add comment to task content and completion record
    #[arg(short, long)]
    pub comment: Option<String>,
}

#[derive(Debug, Args)]
pub struct DeleteCommand {
    /// index from previous list command
    #[arg(value_parser = validate_index)]
    pub index: usize,
}

#[derive(Debug, Args)]
pub struct UpdateCommand {
    /// index from previous list command
    #[arg(value_parser = validate_index)]
    pub index: usize,
    /// update the target time of task,
    /// or event time of record,
    /// or schedule of a recurring task
    #[arg(short, long, value_parser = validate_timestr)]
    pub target_time: Option<String>,
    /// update category of the task/record
    #[arg(short, long)]
    pub category: Option<String>,
    /// replace the content of the task/record
    #[arg(short='w', long)]
    pub content: Option<String>,
    /// add to entry content in a newline 
    #[arg(short, long)]
    pub add_content: Option<String>,
    /// update status of the tasks,
    /// accept ongoing|done|cancelled|duplicate|suspended|pending
    #[arg(short, long, value_parser = parse_status)]
    pub status: Option<u8>
}

#[derive(Debug, Subcommand)]
pub enum ListCommand {
    /// list tasks
    Task(ListTaskCommand),
    /// list records
    Record(ListRecordCommand),
    /// show specific listed item content directly for ease to copy
    Show(ShowContentCommand),
}

#[derive(Debug, Subcommand)]
pub enum OpsCommand {
    /// list statistics
    Stat(OpsStatCommand),
    /// batch update tasks and records, will only proceed after verification.
    Batch(OpsBatchCommand),
    /// backup database to specified path or default location
    Backup(OpsBackupCommand),
}

#[derive(Debug, Args)]
pub struct ListTaskCommand {
    /// task due time. e.g. today,
    /// when present it restrict the task listed to be those,
    /// that are marked for completion prior to this time
    pub timestr: Option<String>,
    /// category of the task
    #[arg(short, long)]
    pub category: Option<String>,
    /// days in the future for tasks to list - mutually exclusive with timestr
    #[arg(short, long, conflicts_with = "timestr")]
    pub days: Option<usize>,
    /// status to list, default to "open",
    /// you can filter individually to ongoing|done|cancelled|duplicate|suspended|pending,
    /// or aggregate status like open|closed|all
    #[arg(short, long, value_parser = parse_status, default_value_t = 254)]
    pub status: u8,
    /// hhow overdue tasks - tasks that are scheduled to be completed in the past,
    /// but were not closed, these tasks are not returned by default
    #[arg(short, long, default_value_t = false)]
    pub overdue: bool,
    /// limit the amount of tasks returned
    #[arg(short, long, default_value_t = 100, value_parser = validate_limit)]
    pub limit: usize,
    /// next page if the previous list command reached limit
    #[arg(short, long, default_value_t = false)]
    pub next_page: bool,
    /// search for tasks containing this text in their content
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListRecordCommand {
    /// category of the record
    #[arg(short, long)]
    pub category: Option<String>,
    /// days of records to retrieve,
    /// e.g. 1 shows record made in the last 24 hours,
    /// value of 7 would show record made in the past week
    #[arg(short, long, conflicts_with_all = ["starting_time", "ending_time"])]
    pub days: Option<usize>,
    /// limit the amount of records returned
    #[arg(short, long, default_value_t = 100, value_parser = validate_limit)]
    pub limit: usize,
    /// list the record starting from this time,
    /// if this is date only, then it is non-inclusive
    #[arg(short, long, value_parser = validate_timestr, conflicts_with = "days")]
    pub starting_time: Option<String>,
    /// list the record ending at this time,
    /// if this is date only, then it is inclusive
    #[arg(short, long, value_parser = validate_timestr, conflicts_with = "days")]
    pub ending_time: Option<String>,
    /// next page if the previous list command reached limit
    #[arg(short, long, default_value_t = false)]
    pub next_page: bool,
    /// search for records containing this text in their content
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Debug, Args)]
pub struct ShowContentCommand {
    /// index from previous list command
    #[arg(value_parser = validate_index)]
    pub index: usize,
}

#[derive(Debug, Args)]
pub struct OpsStatCommand {
    /// specify a particular category to stat
    #[arg(short, long)]
    pub category: Option<String>,
    /// Starting time of the statistic operation
    /// if this is date only, then it is non-inclusive
    #[arg(short, long, value_parser = validate_timestr)]
    pub starting_time: Option<String>,
    /// End time of the statistic operation
    /// if this is date only, then it is inclusive
    #[arg(short, long, value_parser = validate_timestr)]
    pub ending_time: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct OpsBatchCommand {
    /// type of actions for update operation to run on - all or
    /// task|record|recurring_task|recurring_record; For all, only
    /// modifying category is supported
    #[arg(short, long, default_value_t = String::from("all"))]
    pub action: String,
    /// category of the selection at current time
    #[arg(short, long)]
    pub category: Option<String>,
    /// status of the selection at current time
    /// accept ongoing|done|cancelled|duplicate|suspended|pending|open|closed|all
    #[arg(long, value_parser = parse_status)]
    pub status: Option<u8>,
    /// Starting time of the current selection
    /// if this is date only, then it is non-inclusive
    #[arg(short, long, value_parser = validate_timestr)]
    pub starting_time: Option<String>,
    /// End time of the current selection
    /// if this is date only, then it is inclusive
    #[arg(short, long, value_parser = validate_timestr)]
    pub ending_time: Option<String>,
    /// delete the selected items in question
    #[arg(short, long, default_value_t = false)]
    pub delete: bool,
    /// interactively process selected items item by item (y/n)
    #[arg(short, long, default_value_t = false)]
    pub interactive: bool,
    /// modify the current selected items to this category
    #[arg(short = 't', long)]
    pub category_to: Option<String>,
    /// modify the tasks selected to this status
    /// only work with tasks, require concrete status
    /// accept ongoing|done|cancelled|duplicate|suspended|pending
    #[arg(long, value_parser = parse_status)]
    pub status_to: Option<u8>,
    /// bulk update the due date of tasks
    /// only work with action:task
    #[arg(long, value_parser = validate_timestr)]
    pub target_time_to: Option<String>,
}

#[derive(Debug, Args)]
pub struct OpsBackupCommand {
    /// destination path for backup file;
    /// if directory, creates tascli_bak.db inside;
    /// if file path, copies to that path;
    /// if omitted, creates tascli_bak.db in same directory as original.
    #[arg(short, long)]
    pub path: Option<String>,
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
        Err(_) => {
            match parse_recurring_timestr(s) {
                Ok(_) => Ok(s.to_string()),
                Err(e) => Err(e)
            }
        }
    }
}

fn parse_status(s: &str) -> Result<u8, String> {
    match s.to_lowercase().as_str() {
        "ongoing" => Ok(0),
        "done" | "complete" | "completed" => Ok(1),
        "cancelled" | "canceled" | "cancel" => Ok(2),
        "duplicate" => Ok(3),
        "deferred" | "suspended" | "shelved" => Ok(4),
        "removed" | "remove" | "unneeded" | "unnecessary" => Ok(5),
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
