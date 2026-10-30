use chrono::{DateTime, Local};
use crossterm::{
    execute,
    style::{
        Attribute, Attributes, Color, Print, ResetColor, SetAttribute, SetAttributes,
        SetForegroundColor, Stylize,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env,
    fs::{self, OpenOptions},
    io::stdout,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use std::io::Write;

pub const DIR: &'static str = ".todo";
pub const HISTORY_DIR: &'static str = "history";
pub const JOURNAL: &'static str = "journal.jsonl";
pub const FILE: &'static str = "todo.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoJournal {
    pub action: TodoAction,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TodoListConfig {
    pub show_date: bool,
    pub show_progress: bool,
    pub show_completed_range: Option<u64>,
    pub show_details: bool,
}

fn default_config() -> TodoListConfig {
    TodoListConfig {
        show_date: false,
        show_progress: true,
        show_completed_range: Some(36),
        show_details: false,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TodoItem {
    pub timestamp: u64,
    pub completed_timestamp: Option<u64>,
    pub description: String,
    pub tag: String,
    pub detail: String,
    pub done: bool,
}

impl TodoItem {
    pub fn new(description: String, tag: String) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
            completed_timestamp: None,
            description,
            detail: String::new(),
            tag,
            done: false,
        }
    }
}

fn get_time_format(timestamp: u64) -> String {
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

EXAMPLES:
    todo new Buy some milk
    todo -p -t -a
    todo complete
"#;

    println!("{}", help_text);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
    pub path: String,
    pub config: TodoListConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TodoAction {
    Add(TodoItem),
    Complete(Vec<TodoItem>, Vec<TodoItem>),
    Remove(Vec<TodoItem>),
    Edit(Vec<TodoItem>),
    Config(TodoListConfig),
    Revert(u64),
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            path: DIR.to_string(),
            config: default_config(),
        }
    }
    pub fn get_tags(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| item.tag.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn add(&mut self, description: String, tag: String) -> u64 {
        let item = TodoItem::new(description, tag);
        let timestamp = item.timestamp;
        self.items.push(item);
        self.sort();
        timestamp
    }
    pub fn remove(&mut self, timestamps: &mut [u64]) {
        self.items
            .retain(|item| !timestamps.contains(&item.timestamp));
    }
    pub fn save(&mut self, action: TodoAction) -> Result<(), TodoListError> {
        let path = Path::new(&self.path);
        let file = path
            .join(FILE)
            .into_string()
            .unwrap_or(format!("{}/{}", DIR, FILE));
        let history = path.join(HISTORY_DIR);
        let journal_file = path
            .join(JOURNAL)
            .into_string()
            .unwrap_or(format!("{}/{}", DIR, JOURNAL));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;
        let history_file = history
            .join(format!("{}.json", timestamp))
            .into_string()
            .unwrap_or(format!("{}/{}/{}.json", DIR, HISTORY_DIR, timestamp));
        let content =
            serde_json::to_string(self).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        let journal = TodoJournal { action, timestamp };
        let journal_content =
            serde_json::to_string(&journal).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        let mut journal_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(journal_file)
            .map_err(|e| TodoListError::SaveError(e.to_string()))?;
        writeln!(journal_file, "{}", journal_content)
            .map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::create_dir_all(path).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::create_dir_all(history).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::write(history_file, content.clone())
            .map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::write(file, content).map_err(|e| TodoListError::SaveError(e.to_string()))
    }
    pub fn sort(&mut self) {
        self.items.sort_by(|a, b| b.tag.cmp(&a.tag));
    }
    pub fn print(&self) -> usize {
        self.print_highlight(&[], None, None)
    }
    pub fn print_highlight(
        &self,
        to_highlight: &[u64],
        color: Option<Color>,
        attributes: Option<Attributes>,
    ) -> usize {
        let mut lines = 1;
        println!();
        if self.items.is_empty() {
            println!("Todo list empty!");
            return 0;
        }
        for chunk in self.items.chunk_by(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            let done_count = chunk.iter().filter(|i| i.done).count();
            let tag_string = format!(
                "{}{}:",
                if tag.is_empty() { "__untagged__" } else { tag },
                if self.config.show_progress {
                    format!(
                        " [{}/{}] ({}%)",
                        done_count,
                        chunk.len(),
                        ((done_count as f64 / chunk.len() as f64) * 100.0).round()
                    )
                } else {
                    String::new()
                }
            );
            if tag.is_empty() {
                println!(" {}", tag_string.bold().italic().dark_cyan().dim());
            } else {
                println!(" {}", tag_string.bold().italic().cyan());
            }
            lines += 1;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64;
            let mut printed = 0;
            for item in chunk {
                if let Some(range) = self.config.show_completed_range {
                    if item.done && (now - item.completed_timestamp.unwrap_or(0)) >= range * 3600000
                    {
                        continue;
                    }
                };
                printed += 1;
                lines += 1;
                let status = if item.done { "[✔]" } else { "[ ]" };
                let time_string = if self.config.show_date {
                    let timestamp = item.completed_timestamp.unwrap_or(item.timestamp);
                    format!("{:>16}  ", get_time_format(timestamp))
                } else {
                    String::new()
                };
                let item_string = format!(
                    "{} {}{}",
                    status,
                    item.description.to_owned(),
                    (if !item.detail.is_empty() { " [+]" } else { "" })
                        .to_string()
                        .cyan()
                        .bold()
                );
                if to_highlight.iter().any(|s| s == &item.timestamp) {
                    execute!(
                        stdout(),
                        SetAttribute(Attribute::Reset),
                        SetForegroundColor(color.unwrap_or(Color::Reset)),
                        Print(format!("   {}", time_string.clone().italic())),
                        SetForegroundColor(color.unwrap_or(Color::Reset)),
                        SetAttributes(attributes.unwrap_or(Attributes::default())),
                        Print(format!("{}\n", &item_string)),
                        SetAttribute(Attribute::Reset),
                        ResetColor,
                    )
                    .unwrap();
                } else {
                    if item.done {
                        println!(
                            "   {}{}",
                            time_string.clone().italic().green(),
                            item_string.green().bold()
                        );
                    } else {
                        println!("   {}{}", time_string.clone().italic(), item_string);
                    }
                }
                if self.config.show_details && !item.detail.is_empty() {
                    let detail_string = textwrap::fill(
                        &item.detail,
                        textwrap::Options::new(80)
                            .initial_indent(&" ".repeat(7 + time_string.len()))
                            .subsequent_indent(&" ".repeat(7 + time_string.len())),
                    );
                    lines += detail_string.split("\n").count();
                    println!("{}", detail_string.dim().italic());
                }
            }
            if printed < chunk.len() {
                println!(
                    "{}",
                    format!(
                        "{}[-] [{} completed tasks hidden]",
                        " ".repeat(if self.config.show_date { 21 } else { 3 }),
                        chunk.len() - printed
                    )
                    .italic()
                    .dim()
                    .cyan()
                );
                lines += 1;
            }
            println!();
            lines += 1;
        }
        lines
    }
}

#[derive(Debug)]
pub enum TodoListError {
    SaveError(String),
}

pub enum TodoFindError {
    NotFound,
    Invalid(String),
    Other(String),
}

pub enum Command {
    List {
        all: bool,
        detail: bool,
        time: bool,
        pending: bool,
    },
    Filter,
    Detail,
    New(String),
    Remove,
    Complete,
    Edit,
    Journal,
    Config,
    Help,
    Unknown(String),
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
                    Err(e) => return Err(TodoFindError::Other(e.to_string()))
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
        "filter" => return Command::Filter,
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
