use std::{collections::{HashMap, HashSet}, env, fs, process};

mod ui;
use crossterm::style::Stylize;
use serde::{Deserialize, Serialize};

use crate::ui::{Confirm, Text};

const FILE: &'static str = "todo.json";

#[derive(Debug, Serialize, Deserialize)]
struct TodoList {
    items: Vec<TodoItem>,
    path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TodoItem {
    description: String,
    tag: String,
    done: bool,
}

impl TodoList {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            path: FILE.to_string(),
        }
    }
    fn get_tags(&self) -> Vec<String> {
        self.items.iter().map(|item| item.tag.clone()).collect::<HashSet<_>>().into_iter().collect()
    }
    fn push(&mut self, item: TodoItem) {
        self.items.push(item);
        self.sort()
    }
    fn save(&self) -> Result<(), TodoListError> {
        let content =
            serde_json::to_string(self).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::write(self.path.clone(), content).map_err(|e| TodoListError::SaveError(e.to_string()))
    }
    fn sort(&mut self) {
        self.items.sort_by(|a, b| b.tag.cmp(&a.tag));
    }
    fn print(&self) {
        self.print_highlight(None);
    }
    fn print_highlight(&self, to_highlight: Option<TodoItem>) {
        println!();
        if self.items.is_empty() {
            println!("Todo list empty!");
            return;
        }
        for chunk in self.items.chunk_by(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            if tag.is_empty() {
                println!(" {}", format!("{}:", "__untagged__").bold().italic().dark_cyan().dim());
            } else {
                println!(" {}", format!("{}:", tag).bold().italic().cyan());
            }
            for item in chunk {
                let status = if item.done { "[✔]" } else { "[ ]" };
                if to_highlight
                    .as_ref()
                    .is_some_and(|i| i.description == item.description)
                {
                    println!(
                        "{}",
                        format!("   {} {}", status, item.description.to_owned()).bold().yellow().slow_blink()
                    );
                } else {
                    if item.done {
                        println!(
                            "{}",
                            format!("   {} {}", status, item.description.to_owned()).green().bold()
                        );
                    } else {
                        println!("{}", format!("   {} {}", status, item.description.to_owned()));
                    }
                }
            }
            println!()
        }
        println!();
    }
}

#[derive(Debug)]
enum TodoListError {
    SaveError(String),
}

enum TodoFindError {
    NotFound,
    Invalid,
    Other(String),
}

fn find_todo_list() -> Result<TodoList, TodoFindError> {
    let mut current_dir = env::current_dir().map_err(|e| TodoFindError::Other(e.to_string()))?;
    loop {
        let file_path = current_dir.join(FILE);

        if file_path.is_file() {
            let content = fs::read_to_string(file_path).unwrap();
            match serde_json::from_str::<TodoList>(&content) {
                Ok(mut todo_list) => {
                    todo_list.sort();
                    return Ok(todo_list)
                },
                Err(_) => return Err(TodoFindError::Invalid),
            }
        }

        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf()
        } else {
            return Err(TodoFindError::NotFound);
        }
    }
}

enum Command {
    List,
    Filter,
    New { description: String, tag: String },
    NewEmpty,
    Remove,
    Complete,
    Edit,
    Unknown,
}

fn get_command(args: Vec<String>) -> Command {
    if args.len() < 2 {
        return Command::List;
    }

    let mut args = args.into_iter();
    args.next();

    match args.next().unwrap().as_str() {
        "new" | "add" => {
            let mut description = Vec::new();
            while let Some(arg) = args.next() {
                description.push(arg);
            }
            if description.is_empty() {
                return Command::NewEmpty;
            }
            return Command::New {
                description: description.join(" "),
                tag: String::new(),
            };
        }
        "filter" => {
            return  Command::Filter;
        }
        _ => {}
    }

    Command::Unknown
}

fn main() {
    let todo_list = match find_todo_list() {
        Ok(todo_list) => Ok(todo_list),
        Err(err) => {
            match &err {
                TodoFindError::NotFound => {
                    eprintln!("No 'todo.json' found!");
                }
                TodoFindError::Invalid => {
                    eprintln!("'todo.json' found, but it is invalid or corrupted");
                }
                TodoFindError::Other(e) => {
                    eprintln!("Failed to find 'todo.json': {}", e);
                }
            }
            Err(err)
        }
    };

    match get_command(env::args().collect()) {
        Command::List => {
            if let Ok(todo_list) = todo_list {
                todo_list.print();
            }
        }
        Command::NewEmpty => {
            eprintln!("No description provided!")
        }
        Command::New { description, tag } => {
            let mut new_todo_list;
            let mut new_item = TodoItem {
                description,
                tag,
                done: false,
            };
            match todo_list {
                Ok(todo_list) => {
                    new_todo_list = todo_list
                }
                Err(_) => {
                    let selection = Confirm::new("Create new 'todo.json'?")
                        .with_default(true)
                        .prompt();
                    match selection {
                        Ok(true) => {
                            new_todo_list = TodoList::new();
                        }
                        _ => {
                            eprintln!("No 'todo.json' has been created!");
                            process::exit(0)
                        }
                    }
                }
            }
            match Text::new("Enter a Tag for the task")
                .with_default("")
                .with_auto_completion(new_todo_list.get_tags())
                .with_suggest_on_empty(true)
                .prompt()
            {
                Ok(tag) => {
                    new_item.tag = tag
                },
                Err(_) => {
                    eprintln!("No tasks has been added!");
                    process::exit(0)
                }
            }
            new_todo_list.push(new_item.clone());
            new_todo_list.save().unwrap();
            new_todo_list.print_highlight(Some(new_item));
        }
        Command::Filter => {
            if let Ok(mut todo_list) = todo_list {
                match Text::new("Enter Tag filter")
                    .with_default("")
                    .with_auto_completion(todo_list.get_tags())
                    .with_suggest_on_empty(true)
                    .prompt()
                {
                    Ok(tag) => {
                        todo_list.items.retain(|i| i.tag == tag);
                        todo_list.print();
                    }
                    Err(_) => {}
                }
            }
        }
        _ => {}
    }
}
