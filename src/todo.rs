use chrono::{DateTime, Local};
use crossterm::{
    execute, style::{Attribute, Attributes, Color, Print, SetAttribute, SetAttributes, SetForegroundColor, Stylize},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    io::stdout,
    time::{SystemTime, UNIX_EPOCH},
};

const FILE: &'static str = "todo.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoListConfig {
    pub show_date_created: bool,
    pub show_progress: bool,
}

fn default_config() -> TodoListConfig {
    TodoListConfig {
        show_date_created: false,
        show_progress: true,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
    pub path: String,
    pub config: TodoListConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TodoItem {
    pub timestamp: u64,
    pub description: String,
    pub tag: String,
    pub done: bool,
}

impl TodoItem {
    pub fn new(description: String, tag: String) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
            description,
            tag,
            done: false,
        }
    }
}

fn get_time_format(timestamp: u64) -> String {
    if let Some(date) = DateTime::from_timestamp_millis(timestamp as i64) {
        let local_date: DateTime<Local> = DateTime::from(date);
        match (local_date.date_naive() - Local::now().date_naive()).num_days() {
            0 => local_date.format("Today %H:%M").to_string(),
            -1 => local_date.format("Yesterday %H:%M").to_string(),
            x if x > -4 => local_date.format(&format!("{} Days ago %H:%M", x)).to_string(),
            _ => local_date.format("%d-%m-%Y %H:%M").to_string(),
        }
    } else {
        "!err!".to_owned()
    }
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            path: FILE.to_string(),
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
    pub fn save(&self) -> Result<(), TodoListError> {
        let content =
            serde_json::to_string(self).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::write(self.path.clone(), content).map_err(|e| TodoListError::SaveError(e.to_string()))
    }
    pub fn sort(&mut self) {
        self.items.sort_by(|a, b| b.tag.cmp(&a.tag));
    }
    pub fn print(&self) {
        self.print_highlight(&[], None, None);
    }
    pub fn print_highlight(
        &self,
        to_highlight: &[u64],
        color: Option<Color>,
        attributes: Option<Attributes>,
    ) {
        println!();
        if self.items.is_empty() {
            println!("Todo list empty!");
            return;
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
            for item in chunk {
                let status = if item.done { "[✔]" } else { "[ ]" };
                let time_created_string = if self.config.show_date_created {
                    format!("{}  ", get_time_format(item.timestamp))
                } else {
                    String::new()
                };
                let item_string = format!(
                    "{} {}",
                    status,
                    item.description.to_owned()
                );
                if to_highlight.iter().any(|s| s == &item.timestamp) {
                    execute!(
                        stdout(),
                        SetForegroundColor(Color::Reset),
                        SetAttribute(Attribute::Reset),
                        Print(format!("   {}", format!("{:>18}",time_created_string).italic())),
                        SetForegroundColor(color.unwrap_or(Color::Reset)),
                        SetAttributes(attributes.unwrap_or(Attributes::default())),
                        Print(format!("{}\n", &item_string)),
                        SetAttributes(Attributes::none()),
                    )
                    .unwrap();
                } else {
                    if item.done {
                        println!("   {}{}", format!("{:>18}",time_created_string).italic(), item_string.green().bold());
                    } else {
                        println!("   {}{}", format!("{:>18}",time_created_string).italic(), item_string);
                    }
                }
            }
            println!()
        }
    }
}

#[derive(Debug)]
pub enum TodoListError {
    SaveError(String),
}

pub enum TodoFindError {
    NotFound,
    Invalid,
    Other(String),
}

pub enum Command {
    List,
    Filter,
    New(String),
    Remove,
    Complete,
    Edit,
    Config,
    Unknown,
}

pub fn find_todo_list() -> Result<TodoList, TodoFindError> {
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

pub fn get_command(args: Vec<String>) -> Command {
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
            return Command::New(description.join(" "));
        }
        "config" => return Command::Config,
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
