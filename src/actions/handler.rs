use rusqlite::Connection;

use crate::{
    actions::{
        addition,
        list,
        modify,
    },
    args::parser::{
        Action,
        CliArgs,
        ListCommand,
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
        },
    }
}
