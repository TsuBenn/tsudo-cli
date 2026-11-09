use crossterm::{
    execute,
    style::{
        Attribute, Attributes, Color, Print, ResetColor, SetAttribute, SetAttributes,
        SetForegroundColor, Stylize,
    },
};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::stdout,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use std::io::Write;

use crate::todo::{
    models::{TodoAction, TodoItem, TodoJournal, TodoList, TodoListConfig, TodoListError},
    util::{DIR, FILE, HISTORY_DIR, JOURNAL, get_time_format},
};

fn default_config() -> TodoListConfig {
    TodoListConfig {
        show_date: false,
        show_progress: true,
        show_completed_range: Some(6),
        hide_completed_groups: true,
        show_details: false,
        sort_by_completion: false,
        clear_screen: false,
    }
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            path: DIR.to_string().into(),
            config: default_config(),
        }
    }
    pub fn get_tags(&self) -> Vec<String> {
        let mut tags = self.items
            .iter()
            .map(|item| item.tag.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        tags.sort_by(|b, a| self.items.iter().filter(|i| i.tag == *a).max_by_key(|i| i.timestamp).unwrap().timestamp.cmp(&self.items.iter().filter(|i| i.tag == *b).max_by_key(|i| i.timestamp).unwrap().timestamp));
        tags
    }
    pub fn get_completed_tags(&self) -> Vec<String> {
        let mut completed_tags = Vec::new();
        for chunk in self.items.chunk_by(|a, b| a.tag == b.tag) {
            if !chunk.iter().any(|i| !i.done) {
                let mut tag = chunk[0].tag.clone();
                if tag.is_empty() {
                    tag = "__untagged__".to_string()
                }
                completed_tags.push(tag);
            }
        }
        completed_tags
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
            .into_os_string();
        let history = path.join(HISTORY_DIR);
        let gitignore = path.join(".gitignore");
        let journal_file = path
            .join(JOURNAL)
            .into_os_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;
        let history_file = history
            .join(format!("{}.json", timestamp))
            .into_os_string();
        let content =
            serde_json::to_string(self).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        let journal = TodoJournal { action, timestamp };
        let journal_content =
            serde_json::to_string(&journal).map_err(|e| TodoListError::SaveError(e.to_string()))?;

        fs::create_dir_all(path).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::create_dir_all(history).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        let mut journal_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(journal_file)
            .map_err(|e| TodoListError::SaveError(e.to_string()))?;
        writeln!(journal_file, "{}", journal_content)
            .map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::write(history_file, content.clone())
            .map_err(|e| TodoListError::SaveError(e.to_string()))?;
        fs::write(file, content).map_err(|e| TodoListError::SaveError(e.to_string()))?;
        if !gitignore.is_file() {
            fs::write(gitignore, "journal.jsonl\nhistory/")
                .map_err(|e| TodoListError::SaveError(e.to_string()))?;
        }
        Ok(())
    }
    pub fn sort_by_completion(&mut self) {
        self.sort();
        let mut group_tags = self.items.chunk_by(|a, b| a.tag == b.tag).map(|chunk| chunk.to_vec()).collect::<Vec<_>>();
        for group in group_tags.iter_mut() {
            group.sort_by(|a, b| a.done.cmp(&b.done));
        }
        self.items = group_tags.concat()
    }
    pub fn sort(&mut self) {
        self.items.sort_by(|a, b| a.tag.cmp(&b.tag));
        let mut group_tags = self.items.chunk_by(|a, b| a.tag == b.tag).map(|chunk| chunk.to_vec()).collect::<Vec<_>>();
        group_tags.sort_by(|a, b| b.iter().max_by_key(|i| i.timestamp).unwrap().timestamp.cmp(&a.iter().max_by_key(|i| i.timestamp).unwrap().timestamp));
        group_tags.sort_by(|b, a| a.iter().any(|i| !i.done).cmp(&b.iter().any(|i| !i.done)));
        self.items = group_tags.concat()
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
        let mut this = self.clone();
        if self.config.sort_by_completion {
            this.sort_by_completion();
        }
        let mut lines = 1;
        println!();
        if this.items.is_empty() {
            println!("Todo list empty!");
            return 0;
        }
        for chunk in this.items.chunk_by(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            let done_count = chunk.iter().filter(|i| i.done).count();
            let pct_string = format!(
                "({}%)",
                ((done_count as f64 / chunk.len() as f64) * 100.0).round()
            );
            let fully_completed = !chunk.iter().any(|i| !i.done);
            let tag_string = format!(
                "{}{}{}",
                if tag.is_empty() { "__untagged__" } else { tag },
                if self.config.show_progress {
                    format!(
                        " [{}/{}] {}",
                        done_count,
                        chunk.len(),
                        if done_count == chunk.len() {
                            pct_string.green().to_string()
                        } else {
                            pct_string
                        }
                    )
                } else {
                    String::new()
                },
                if self.config.hide_completed_groups && fully_completed {
                    " [Hidden]".dim().cyan().italic().to_string()
                } else {
                    ":".to_string()
                }
            );
            println!(" {}", tag_string.bold().italic().cyan());
            lines += 1;
            if self.config.hide_completed_groups && fully_completed && !chunk.iter().any(|s| to_highlight.contains(&s.timestamp)) {
                continue;
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64;
            let mut printed = 0;
            for item in chunk {
                let highlighted = to_highlight.iter().any(|s| s == &item.timestamp);
                if !highlighted {
                    if self.config.hide_completed_groups && fully_completed {
                        continue;
                    }
                    if let Some(range) = self.config.show_completed_range {
                        if item.done && (now - item.completed_timestamp.unwrap_or(0)) >= range * 3600000
                        {
                            continue;
                        }
                    };
                }
                printed += 1;
                lines += 1;
                let status = if item.done { "[✓]" } else { "[ ]" };
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
                if highlighted {
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
