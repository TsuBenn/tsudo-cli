use chrono::{DateTime, Local};
use std::{
    env,
    fs::{self},
};

use crate::todo::models::{TodoFindError, TodoList, Command};

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

pub fn print_help() {
    let help_text = r#"
todo - Lightweight, Quick, Project-Scoped Task Management

USAGE:
    todo [COMMAND]... [ARGS]...

CORE:
    todo
        <none>
            List all Tasks, following the project's config

        -a, --all
            List all Tasks, even hidden ones
            This will override the config behavior

        -d, --detail
            List all Tasks, including their details

        -t, --time
            List all Tasks, including their creation time or
            completion time
            This will override the config behavior

        -p, --pending
            List only pending Tasks
            This will override the -a behavior

    todo add/new <desc>
            Add a new Tasks with the provided description
            If description is not provided, todo will prompt
            the user to type in one.
            The user will also be prompted to give the task a
            tag, but it can be left blank.

INTERACTIVE:
    todo complete
            Toggles Tasks completion status

    todo remove
            Select a Task or multiple Tasks for removal

    todo edit
            Select a Task or multiple Tasks for editing
            Tags can be editted in bulk, while description and
            detail can only be editted individually

    todo detail
            Select a Task or multiple Tasks and how their details
            Details include Description, Tag, Creation Time,
            Completion Time and Detail

PROJECT:
    todo config
            Edit the default behavior of 'todo'
            Config includes:
                Show Creation/Completion Time of tasks
                Show Progress Tasks Groups (Grouped by Tags)
                Show Completed Tasks within a Range (hour)
                Hide Completed Task Groups
                Sort Tasks by Completion

EXAMPLES:
    todo new Buy some milk
    todo -p -t -a
    todo complete
"#;

    println!("{}", help_text);
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
            return Command::Filter(query.join(" "));
        }
        "detail" => return Command::Detail,
        "help" | "-h" | "--help" => return Command::Help,
        "journal" | "-j" => return Command::Journal,
        arg => {
            let mut arg = arg.to_owned();
            let mut all = false;
            let mut detail = false;
            let mut time = false;
            let mut pending = false;
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
                            todo_list.path = dir_path.into_string().unwrap_or(DIR.to_string());
                            todo_list.sort();
                            return Ok(todo_list);
                        }
                        Err(_) => {
                            return Err(TodoFindError::Invalid(
                                file_path.into_string().unwrap_or(FILE.to_string()),
                            ));
                        }
                    },
                    Err(e) => return Err(TodoFindError::Other(e.to_string())),
                }
            } else {
                let mut todo_list = TodoList::new();
                todo_list.path = file_path.into_string().unwrap_or(FILE.to_string());
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
