use std::{env, process};

mod cli;
mod todo;

use crate::todo::{Command, TodoFindError, TodoList, find_todo_list, get_command};

fn run(args: Vec<String>, todo_list: Result<TodoList, TodoFindError>) -> Result<(), i32> {
    match get_command(args) {
        Command::List {
            all,
            detail,
            time,
            pending,
        } => Ok(cli::list(todo_list, all, detail, time, pending)),
        Command::New(description) => cli::new(todo_list, description),
        Command::Filter(input_tag) => cli::filter(todo_list, input_tag),
        Command::Complete => cli::complete(todo_list),
        Command::Edit => cli::edit(todo_list),
        Command::Remove => cli::remove(todo_list),
        Command::Config => cli::config(todo_list),
        Command::Detail => cli::detail(todo_list),
        Command::Help(command) => Ok(todo::print_help(command)),
        Command::Unknown(cmd) => {
            eprintln!(
                "\n  Error: No such command `{cmd}`\n  Try --help to see the available commands\n",
            );
            Err(1)
        }
        Command::Journal => cli::journal(todo_list),
    }
}

fn main() {
    let todo_list = match find_todo_list() {
        Ok(todo_list) => Ok(todo_list),
        Err(err) => {
            match &err {
                TodoFindError::NotFound => {
                    eprintln!("\n  Not .todo found in the current directory (or any parent directory)!\n");
                }
                TodoFindError::Invalid(_) => {
                    eprintln!("\n  .todo found, but it is invalid or corrupted!\n");
                }
                TodoFindError::Other(e) => {
                    eprintln!("\n  Failed to find .todo: {}\n", e);
                }
            }
            Err(err)
        }
    };

    if let Err(i) = run(env::args().collect(), todo_list) {
        process::exit(i)
    }
}
