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
        Action::Task(cmd) => {
            println!("Handling task command: {:?}", cmd);
            addition::handle_taskcmd(conn, &cmd)
        }
        Action::Record(cmd) => {
            println!("Handling record command: {:?}", cmd);
            addition::handle_recordcmd(conn, &cmd)
        }
        Action::Done(cmd) => {
            println!("Handling done command: {:?}", cmd);
            modify::handle_donecmd(conn, &cmd)
        }
        Action::Update(cmd) => {
            println!("Handling update command: {:?}", cmd);
            modify::handle_updatecmd(conn, &cmd)
        }
        Action::List(list_cmd) => match list_cmd {
            ListCommand::Task(cmd) => {
                println!("Handling list task command: {:?}", cmd);
                list::handle_listtasks(conn, cmd)
            }
            ListCommand::Record(cmd) => {
                println!("Handling list task command: {:?}", cmd);
                list::handle_listrecords(conn, cmd)
            }
        },
    }
}
