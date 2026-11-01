use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub hide_completed_groups: bool,
    pub show_details: bool,
    pub sort_by_completion: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    EditBulk {
        edited: Vec<TodoItem>,
        old_tag: String,
        new_tag: String,
    },
    EditSingular {
        edited: TodoItem,
        old_desc: String,
        new_desc: Option<String>,
        old_detail: String,
        new_detail: Option<String>,
        old_tag: String,
        new_tag: Option<String>,
    },
    Config {
        old: TodoListConfig,
        new: TodoListConfig,
    },
    Revert(u64),
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
    Filter(String),
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
