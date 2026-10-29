use std::{collections::HashSet, env, fs, process};

mod ui;
use crossterm::style::{Color, Stylize};
use serde::{Deserialize, Serialize};

use crate::ui::{Confirm, Select, Text};

const FILE: &'static str = "todo.json";
const PAGE_SIZE: usize = 20;

#[derive(Debug, Serialize, Deserialize)]
struct TodoList {
    items: Vec<TodoItem>,
    path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
        self.items
            .iter()
            .map(|item| item.tag.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
    fn push(&mut self, item: TodoItem) {
        self.items.push(item);
        self.sort()
    }
    fn remove(&mut self, indices: &mut [usize]) {
        indices.sort_by(|a, b| b.cmp(&a));
        for i in indices {
            self.items.remove(*i);
        }
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
        self.print_highlight(vec![]);
    }
    fn print_remove(&self, to_remove: &[usize]) {
        println!();
        if self.items.is_empty() {
            println!("Todo list empty!");
            return;
        }
        let mut i = 0;
        for chunk in self.items.chunk_by(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            if tag.is_empty() {
                println!(
                    " {}",
                    format!("{}:", "__untagged__")
                        .bold()
                        .italic()
                        .dark_cyan()
                        .dim()
                );
            } else {
                println!(" {}", format!("{}:", tag).bold().italic().cyan());
            }
            for item in chunk {
                let status = if item.done { "[✔]" } else { "[ ]" };
                if to_remove.contains(&i) {
                    println!(
                        "   {}",
                        format!("{} {}", status, item.description.to_owned())
                            .crossed_out()
                            .red()
                    );
                } else {
                    if item.done {
                        println!(
                            "{}",
                            format!("   {} {}", status, item.description.to_owned())
                                .green()
                                .bold()
                        );
                    } else {
                        println!(
                            "{}",
                            format!("   {} {}", status, item.description.to_owned())
                        );
                    }
                }
                i += 1;
            }
            println!()
        }
    }
    fn print_highlight(&self, to_highlight: Vec<TodoItem>) {
        println!();
        if self.items.is_empty() {
            println!("Todo list empty!");
            return;
        }
        for chunk in self.items.chunk_by(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            if tag.is_empty() {
                println!(
                    " {}",
                    format!("{}:", "__untagged__")
                        .bold()
                        .italic()
                        .dark_cyan()
                        .dim()
                );
            } else {
                println!(" {}", format!("{}:", tag).bold().italic().cyan());
            }
            for item in chunk {
                let status = if item.done { "[✔]" } else { "[ ]" };
                if to_highlight.contains(item)
                {
                    println!(
                        "{}",
                        format!("   {} {}", status, item.description.to_owned())
                            .bold()
                            .yellow()
                            .slow_blink()
                    );
                } else {
                    if item.done {
                        println!(
                            "{}",
                            format!("   {} {}", status, item.description.to_owned())
                                .green()
                                .bold()
                        );
                    } else {
                        println!(
                            "{}",
                            format!("   {} {}", status, item.description.to_owned())
                        );
                    }
                }
            }
            println!()
        }
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
            let content = fs::read_to_string(&file_path).unwrap();
            match serde_json::from_str::<TodoList>(&content) {
                Ok(mut todo_list) => {
                    todo_list.path = file_path.into_string().unwrap_or(FILE.to_string());
                    todo_list.sort();
                    return Ok(todo_list);
                }
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
        "complete" | "done" => return Command::Complete,
        "edit" => return Command::Edit,
        "remove" | "delete" | "rm" => return Command::Remove,
        "filter" => {
            return Command::Filter;
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
                Ok(todo_list) => new_todo_list = todo_list,
                Err(_) => {
                    let selection = Confirm::new("Create new 'todo.json'?")
                        .with_default(true)
                        .prompt();
                    match selection {
                        Ok(true) => {
                            new_todo_list = TodoList::new();
                        }
                        Err(ui::UIError::Canceled) | Ok(false) => {
                            eprintln!("No 'todo.json' has been created!");
                            process::exit(0)
                        }
                        Err(ui::UIError::Terminal(err)) => {
                            eprintln!("Terminal error: {err}");
                            process::exit(1)
                        }
                        Err(ui::UIError::Other(err)) => {
                            eprintln!("Error: {err}");
                            process::exit(1)
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
                Ok(tag) => new_item.tag = tag,
                Err(ui::UIError::Canceled) => {
                    eprintln!("No tasks has been added!");
                    process::exit(0)
                }
                Err(ui::UIError::Terminal(err)) => {
                    eprintln!("Terminal error: {err}");
                    process::exit(1)
                }
                Err(ui::UIError::Other(err)) => {
                    eprintln!("Error: {err}");
                    process::exit(1)
                }
            }
            new_todo_list.push(new_item.clone());
            match new_todo_list.save() {
                Ok(()) => {}
                _ => {
                    eprintln!("Problem with saving to 'todo.json'!");
                    eprintln!("No change has been made!");
                    process::exit(1)
                }
            }
            new_todo_list.print_highlight(vec![new_item]);
        }
        Command::Filter => {
            if let Ok(mut todo_list) = todo_list {
                let mut tag_select = Select::<String>::new("Enter Tag filter", |i| {
                    if i.is_empty() {
                        "__untagged__".to_string()
                    } else {
                        i.clone()
                    }
                });
                let mut tags = todo_list.get_tags();
                tag_select.add_items(tags.as_mut_slice(), None);
                match tag_select.prompt() {
                    Ok(result) => {
                        todo_list.items.retain(|i| i.tag == *result.selected[0].1);
                        todo_list.print();
                    }
                    Err(ui::UIError::Canceled) => {}
                    Err(ui::UIError::Terminal(err)) => {
                        eprintln!("Terminal error: {err}");
                        process::exit(1)
                    }
                    Err(ui::UIError::Other(err)) => {
                        eprintln!("Error: {err}");
                        process::exit(1)
                    }
                }
            }
        }
        Command::Complete => {
            if let Ok(mut todo_list) = todo_list {
                let mut done_multi_selection =
                    Select::<TodoItem>::new("Toggle tasks completion status", |i| {
                        i.description.clone()
                    })
                    .with_multi_selection(true)
                    .with_group_items(true)
                    .with_page_size(PAGE_SIZE)
                    .with_initial_selection(
                        todo_list
                            .items
                            .iter()
                            .enumerate()
                            .filter_map(|(i, item)| if item.done { Some(i) } else { None })
                            .collect(),
                    );
                for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
                    let tag = &chunk[0].tag;
                    let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
                    done_multi_selection.add_items(chunk, Some(&group));
                }
                match done_multi_selection.prompt() {
                    Ok(result) => {
                        for (_, item) in result.selected {
                            item.done = true
                        }
                        for (_, item) in result.unselected {
                            item.done = false
                        }
                        match todo_list.save() {
                            Ok(()) => {}
                            _ => {
                                eprintln!("Problem with saving to 'todo.json'!");
                                eprintln!("No change has been made!");
                                process::exit(1)
                            }
                        }
                        todo_list.print();
                    }
                    Err(ui::UIError::Canceled) => {
                        eprintln!("No completion status of tasks has been changed!");
                        process::exit(0)
                    }
                    Err(ui::UIError::Terminal(err)) => {
                        eprintln!("Terminal error: {err}");
                        process::exit(1)
                    }
                    Err(ui::UIError::Other(err)) => {
                        eprintln!("Error: {err}");
                        process::exit(1)
                    }
                }
            }
        }
        Command::Edit => {
            if let Ok(mut todo_list) = todo_list {
                let mut edit_select =
                    Select::<TodoItem>::new("Select a task or multiple tasks to edit", |i| {
                        let status = if i.done { "(✓)" } else { "( )" };
                        format!("{} {}", status, i.description)
                    })
                    .with_multi_selection(true)
                    .with_select_on_enter(true)
                    .with_group_items(true)
                    .with_page_size(PAGE_SIZE);
                let tags = todo_list.get_tags();
                for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
                    let tag = &chunk[0].tag;
                    let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
                    edit_select.add_items(chunk, Some(&group));
                }
                match edit_select.prompt() {
                    Ok(mut task_result) => {
                        let multi_selected = task_result.selected.len() > 1;
                        let mut field_select =
                            Select::<String>::new("Select a field to edit", |i| i.clone())
                                .with_disabled_items(if multi_selected { vec![0] } else { vec![] });
                        let mut options = vec!["Description".to_owned(), "Tag".to_owned()];
                        field_select.add_items(options.as_mut_slice(), None);
                        let mut edited_item = vec![];
                        match field_select.prompt() {
                            Ok(result) => match result.index() {
                                0 => {
                                    let selected = task_result.data_mut();
                                    match Text::new("Enter new task description")
                                        .with_default(&selected.description)
                                        .with_initial_value(&selected.description)
                                        .prompt()
                                    {
                                        Ok(new_description) => {
                                            selected.description = new_description;
                                            edited_item.push(selected.clone());
                                        }
                                        Err(ui::UIError::Canceled) => {
                                            eprintln!("No tasks has been edited!");
                                            process::exit(0)
                                        }
                                        Err(ui::UIError::Terminal(err)) => {
                                            eprintln!("Terminal error: {err}");
                                            process::exit(1)
                                        }
                                        Err(ui::UIError::Other(err)) => {
                                            eprintln!("Error: {err}");
                                            process::exit(1)
                                        }
                                    }
                                }
                                1 => {
                                    if multi_selected {
                                        match Text::new("Enter new tasks Tag")
                                            .with_auto_completion(tags)
                                            .with_suggest_on_empty(true)
                                            .prompt()
                                        {
                                            Ok(new_tag) => {
                                                for (_, item) in task_result.selected {
                                                    item.tag = new_tag.clone();
                                                    edited_item.push(item.clone());
                                                }
                                            },
                                            Err(ui::UIError::Canceled) => {
                                                eprintln!("No tasks has been edited!");
                                                process::exit(0)
                                            }
                                            Err(ui::UIError::Terminal(err)) => {
                                                eprintln!("Terminal error: {err}");
                                                process::exit(1)
                                            }
                                            Err(ui::UIError::Other(err)) => {
                                                eprintln!("Error: {err}");
                                                process::exit(1)
                                            }
                                        }
                                    } else {
                                        let selected = task_result.data_mut();
                                        match Text::new("Enter new task Tag")
                                            .with_default(&selected.tag)
                                            .with_initial_value(&selected.tag)
                                            .with_auto_completion(tags)
                                            .with_suggest_on_empty(true)
                                            .prompt()
                                        {
                                            Ok(new_tag) => {
                                                selected.tag = new_tag;
                                                edited_item.push(selected.clone());
                                            },
                                            Err(ui::UIError::Canceled) => {
                                                eprintln!("No tasks has been edited!");
                                                process::exit(0)
                                            }
                                            Err(ui::UIError::Terminal(err)) => {
                                                eprintln!("Terminal error: {err}");
                                                process::exit(1)
                                            }
                                            Err(ui::UIError::Other(err)) => {
                                                eprintln!("Error: {err}");
                                                process::exit(1)
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            },
                            Err(ui::UIError::Canceled) => {
                                eprintln!("No tasks has been edited!");
                                process::exit(0)
                            }
                            Err(ui::UIError::Terminal(err)) => {
                                eprintln!("Terminal error: {err}");
                                process::exit(1)
                            }
                            Err(ui::UIError::Other(err)) => {
                                eprintln!("Error: {err}");
                                process::exit(1)
                            }
                        }
                        todo_list.sort();
                        match todo_list.save() {
                            Ok(()) => {}
                            _ => {
                                eprintln!("Problem with saving to 'todo.json'!");
                                eprintln!("No change has been made!");
                                process::exit(1)
                            }
                        }
                        todo_list.print_highlight(edited_item);
                    }
                    Err(ui::UIError::Canceled) => {
                        eprintln!("No tasks has been edited!");
                        process::exit(0)
                    }
                    Err(ui::UIError::Terminal(err)) => {
                        eprintln!("Terminal error: {err}");
                        process::exit(1)
                    }
                    Err(ui::UIError::Other(err)) => {
                        eprintln!("Error: {err}");
                        process::exit(1)
                    }
                }
            }
        }
        Command::Remove => {
            if let Ok(mut todo_list) = todo_list {
                let mut remove_multi_selection =
                    Select::<TodoItem>::new("Select tasks for removal", |i| {
                        let status = if i.done { "(✓)" } else { "( )" };
                        format!("{} {}", status, i.description)
                    })
                    .with_multi_selection(true)
                    .with_group_items(true)
                    .with_page_size(PAGE_SIZE)
                    .with_selected_color(Color::Red);
                for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
                    let tag = &chunk[0].tag;
                    let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
                    remove_multi_selection.add_items(chunk, Some(&group));
                }
                let mut to_remove: Vec<usize>;
                match remove_multi_selection.prompt() {
                    Ok(result) => {
                        to_remove = result.selected.iter().map(|(i, _)| *i).collect();
                    }
                    Err(ui::UIError::Canceled) => {
                        eprintln!("No tasks has been removed!");
                        process::exit(0)
                    }
                    Err(ui::UIError::Terminal(err)) => {
                        eprintln!("Terminal error: {err}");
                        process::exit(1)
                    }
                    Err(ui::UIError::Other(err)) => {
                        eprintln!("Error: {err}");
                        process::exit(1)
                    }
                }
                todo_list.print_remove(&to_remove);
                todo_list.remove(&mut to_remove);
                match todo_list.save() {
                    Ok(()) => {}
                    _ => {
                        eprintln!("Problem with saving to 'todo.json'!");
                        eprintln!("No change has been made!");
                        process::exit(1)
                    }
                }
            }
        }
        _ => {}
    }
}
