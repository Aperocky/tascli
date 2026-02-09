use rusqlite::Connection;

use crate::{
    actions::{
        addition,
        list,
        modify,
        ops,
    },
    args::parser::{
        Action,
        CliArgs,
        ListCommand,
        OpsCommand,
    },
};

pub fn handle_commands(conn: &Connection, args: CliArgs) -> Result<(), String> {
    match args.arguments {
        Action::Task(cmd) => addition::handle_taskcmd(conn, &cmd),
        Action::Record(cmd) => addition::handle_recordcmd(conn, &cmd),
        Action::Done(cmd) => modify::handle_donecmd(conn, &cmd),
        Action::Delete(cmd) => modify::handle_deletecmd(conn, &cmd),
        Action::Update(cmd) => modify::handle_updatecmd(conn, &cmd),
        Action::List(list_cmd) => match list_cmd {
            ListCommand::Task(cmd) => list::handle_listtasks(conn, cmd),
            ListCommand::Record(cmd) => list::handle_listrecords(conn, cmd),
            ListCommand::Show(cmd) => list::handle_showcontent(conn, cmd),
        },
        Action::Ops(ops_cmd) => match ops_cmd {
            OpsCommand::Stat(cmd) => ops::handle_statcmd(conn, &cmd),
            OpsCommand::Batch(_) => Err("ops batch not yet implemented".to_string()),
        },
    }
}
