use chrono::{DateTime, Local};
use std::{
    env,
    fs::{self},
};

use crate::todo::{
    help_texts::{
        COMPLETE_HELP, CONFIG_HELP, DETAIL_HELP, EDIT_HELP, FILTER_HELP, GENERAL_HELP,
        JOURNAL_HELP, LIST_HELP, NEW_HELP, REMOVE_HELP,
    },
    models::{Command, TodoFindError, TodoList},
};

pub const DIR: &'static str = ".todo";
pub const HISTORY_DIR: &'static str = "history";
pub const JOURNAL: &'static str = "journal.jsonl";
pub const FILE: &'static str = "todo.json";

pub fn get_time_format(timestamp: u64) -> String {
    if let Some(date) = DateTime::from_timestamp_millis(timestamp as i64) {
        let local_date: DateTime<Local> = DateTime::from(date);
        let x = (Local::now().timestamp_millis() - local_date.timestamp_millis()) / 1000;
        if x < 60 {
            format!("Just now")
        } else if x < 3600 {
            format!("{}m ago", x / 60)
        } else if x < 24 * 3600 {
            format!("{}h ago", x / 3600)
        } else if x < 24 * 3600 * 2 {
            local_date.format("Yesterday %H:%M").to_string()
        } else {
            local_date.format("%d-%m-%Y %H:%M").to_string()
        }
    } else {
        "!err!".to_owned()
    }
}

pub fn print_help(cmd: Option<String>) {
    let mut text = GENERAL_HELP;
    if let Some(cmd) = cmd {
        match cmd.as_str() {
            "list" => text = LIST_HELP,
            "add" | "new" => text = NEW_HELP,
            "config" => text = CONFIG_HELP,
            "complete" | "done" => text = COMPLETE_HELP,
            "edit" => text = EDIT_HELP,
            "remove" | "delete" | "rm" => text = REMOVE_HELP,
            "filter" => text = FILTER_HELP,
            "detail" => text = DETAIL_HELP,
            "help" | "-h" | "--help" => text = GENERAL_HELP,
            "journal" | "log" => text = JOURNAL_HELP,
            _ => {
                eprintln!(
                    "\n  Error: No such command `{cmd}`\n  Try --help to see the available commands\n"
                );
                return;
            }
        }
    }
    println!("{}", text);
}

pub fn get_command(args: Vec<String>) -> Command {
    if args.len() < 2 {
        return Command::List {
            all: false,
            detail: false,
            time: false,
            pending: false,
        };
    }

    let mut args = args.into_iter();
    args.next();

    match args.next().unwrap().as_str() {
        "new" | "add" => {
            let mut description = Vec::new();
            while let Some(arg) = args.next() {
                description.push(arg);
            }
            return Command::New(description.join(" "));
        }
        "config" => return Command::Config,
        "complete" | "done" => return Command::Complete,
        "edit" => return Command::Edit,
        "remove" | "delete" | "rm" => return Command::Remove,
        "filter" => {
            let mut query = Vec::new();
            while let Some(arg) = args.next() {
                query.push(arg);
            }
            return Command::Filter(query);
        }
        "detail" => return Command::Detail,
        "help" | "-h" | "--help" => return Command::Help(args.next()),
        "journal" | "log" => return Command::Journal,
        arg => {
            let mut arg = arg.to_owned();
            let mut all = false;
            let mut detail = false;
            let mut time = false;
            let mut pending = false;
            if arg == "list" {
                if let Some(next_arg) = args.next() {
                    arg = next_arg;
                } else {
                    return Command::List {
                        all: false,
                        detail: false,
                        time: false,
                        pending: false,
                    };
                }
            }
            loop {
                match arg.as_str() {
                    "--all" | "-a" => all = true,
                    "--detail" | "-d" => detail = true,
                    "--time" | "-t" => time = true,
                    "--pending" | "-p" => pending = true,
                    _ => return Command::Unknown(arg),
                }
                match args.next() {
                    Some(next_arg) => arg = next_arg,
                    None => break,
                }
            }
            return Command::List {
                all,
                detail,
                time,
                pending,
            };
        }
    }
}

pub fn find_todo_list() -> Result<TodoList, TodoFindError> {
    let mut current_dir = env::current_dir().map_err(|e| TodoFindError::Other(e.to_string()))?;
    loop {
        let dir_path = current_dir.join(DIR);

        if dir_path.is_dir() {
            let file_path = dir_path.join(FILE);
            if file_path.is_file() {
                match fs::read_to_string(&file_path) {
                    Ok(content) => match serde_json::from_str::<TodoList>(&content) {
                        Ok(mut todo_list) => {
                            todo_list.path = dir_path.into_os_string();
                            todo_list.sort();
                            return Ok(todo_list);
                        }
                        Err(_) => {
                            return Err(TodoFindError::Invalid(dir_path.into_os_string()));
                        }
                    },
                    Err(e) => return Err(TodoFindError::Other(e.to_string())),
                }
            } else {
                let mut todo_list = TodoList::new();
                todo_list.path = dir_path.into_os_string();
                return Ok(todo_list);
            }
        }

        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf()
        } else {
            return Err(TodoFindError::NotFound);
        }
    }
}
