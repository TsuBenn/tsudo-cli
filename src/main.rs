use std::{
    env, fs::{self, File}, io::{BufRead, BufReader, ErrorKind::NotFound}, path::Path, process, time::{SystemTime, UNIX_EPOCH},
};

mod todo;

use chrono::{DateTime, Local};
use crossterm::style::{Attribute, Attributes, Color, Stylize};

use tsudo::ui::{self, Confirm, Select, Text, clean_line, clean_lines};

use crate::todo::*;

const PAGE_SIZE: usize = 30;
const DONE_PREFIX: &'static str = "✓";
const PENDING_PREFIX: &'static str = " ";

fn handle_ui_error(err: ui::UIError, cancel_message: &str) -> i32 {
    match err {
        ui::UIError::Canceled => {
            eprintln!("{}", cancel_message);
            0
        }
        ui::UIError::Terminal(err) => {
            eprintln!("Terminal error: {err}");
            1
        }
        ui::UIError::Other(err) => {
            eprintln!("Error: {err}");
            1
        }
    }
}

fn handle_save_error(err: TodoListError) -> i32 {
    match err {
        TodoListError::SaveError(e) => {
            eprintln!("Problem with saving to .todo! {e}");
            eprintln!("No change has been made!");
            1
        }
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() > max {
        let mut text: String = text.chars().take(max - 1).collect();
        text.push('…');
        return text;
    }
    text.to_string()
}

fn run(args: Vec<String>, todo_list: Result<TodoList, TodoFindError>) {
    match get_command(args) {
        Command::List {
            all,
            detail,
            time,
            pending,
        } => {
            if let Ok(mut todo_list) = todo_list {
                if all {
                    todo_list.config.show_completed_range = None
                }
                if detail {
                    todo_list.config.show_details = true
                }
                if time {
                    todo_list.config.show_date = true
                }
                if pending {
                    todo_list.config.show_completed_range = Some(0)
                }
                todo_list.print();
            }
        }
        Command::New(mut description) => {
            let mut new_todo_list;
            match todo_list {
                Ok(todo_list) => new_todo_list = todo_list,
                Err(TodoFindError::Invalid(file_path)) => {
                    let selection = Confirm::new("Override current .todo?")
                        .with_default(true)
                        .prompt();
                    match selection {
                        Ok(true) => {
                            new_todo_list = TodoList::new();
                            new_todo_list.path = file_path
                        }
                        Ok(false) => process::exit(handle_ui_error(
                            ui::UIError::Canceled,
                            "No change has been made!",
                        )),
                        Err(err) => process::exit(handle_ui_error(err, "No change has been made!")),
                    }
                }
                Err(_) => {
                    let selection = Confirm::new("Create new .todo?")
                        .with_default(true)
                        .prompt();
                    match selection {
                        Ok(true) => {
                            new_todo_list = TodoList::new();
                        }
                        Ok(false) => process::exit(handle_ui_error(
                            ui::UIError::Canceled,
                            "No 'todo.json' has been created!",
                        )),
                        Err(err) => {
                            process::exit(handle_ui_error(err, "No .todo has been created!"))
                        }
                    }
                }
            }
            if description.is_empty() {
                let new_description = Text::new("Enter a description for the Task").prompt();
                match new_description {
                    Ok(new_description) => description = new_description,
                    Err(err) => process::exit(handle_ui_error(err, "No Task has been added!")),
                }
            }
            let tag = match Text::new("Enter a Tag for the Task")
                .with_default("")
                .with_auto_completion(new_todo_list.get_tags())
                .with_suggest_on_empty(true)
                .prompt()
            {
                Ok(tag) => tag,
                Err(err) => process::exit(handle_ui_error(err, "No Task has been added!")),
            };
            let new_item_timestamp = new_todo_list.add(description.clone(), tag.clone());
            if let Err(err) = new_todo_list.save(TodoAction::Add(TodoItem::new(description, tag))) {
                process::exit(handle_save_error(err))
            }
            new_todo_list.print_highlight(
                &[new_item_timestamp],
                Some(Color::Yellow),
                Some(Attributes::default().with(Attribute::Bold)),
            );
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
                    Err(err) => process::exit(handle_ui_error(err, "")),
                }
            }
        }
        Command::Complete => {
            if let Ok(mut todo_list) = todo_list {
                let old_completed_timestamps: Vec<u64> = todo_list
                    .items
                    .iter()
                    .filter_map(|i| if i.done { Some(i.timestamp) } else { None })
                    .collect();
                let mut done_multi_selection =
                    Select::<TodoItem>::new("Toggle Tasks completion status", |i| {
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
                    Ok(mut result) => {
                        let mut completed_item = vec![];
                        let mut uncompleted_item = vec![];
                        let mut changed_timestamps: Vec<u64> = result
                            .selected
                            .iter_mut()
                            .filter_map(|(_, item)| {
                                if !old_completed_timestamps.contains(&item.timestamp) {
                                    item.completed_timestamp = Some(
                                        SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .expect("Time went backwards")
                                            .as_millis()
                                            as u64,
                                    );
                                    completed_item.push(item.clone());
                                    Some(item.timestamp)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        changed_timestamps.append(
                            &mut result
                                .unselected
                                .iter_mut()
                                .filter_map(|(_, item)| {
                                    if old_completed_timestamps.contains(&item.timestamp) {
                                        item.completed_timestamp = None;
                                        uncompleted_item.push(item.clone());
                                        Some(item.timestamp)
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                        );
                        for (_, item) in result.selected {
                            item.done = true
                        }
                        for (_, item) in result.unselected {
                            item.done = false
                        }
                        if let Err(err) =
                            todo_list.save(TodoAction::Complete(completed_item, uncompleted_item))
                        {
                            process::exit(handle_save_error(err))
                        }
                        todo_list.print_highlight(
                            &changed_timestamps,
                            Some(Color::Yellow),
                            Some(Attributes::default().with(Attribute::Bold)),
                        );
                    }
                    Err(err) => process::exit(handle_ui_error(
                        err,
                        "No completion status of Tasks has been changed!",
                    )),
                }
            }
        }
        Command::Edit => {
            if let Ok(mut todo_list) = todo_list {
                loop {
                    let mut edit_select =
                        Select::<TodoItem>::new("Select a Task or multiple Tasks to edit", |i| {
                            let status = if i.done { DONE_PREFIX } else { PENDING_PREFIX };
                            let detail_status = if i.detail.is_empty() {
                                "".reset()
                            } else {
                                " [+]".cyan().bold()
                            };
                            format!(" {} {}{}", status, i.description, detail_status)
                        })
                        .with_multi_selection(true)
                        .with_hybrid(true)
                        .with_finished_formatter(|item| item.description.clone())
                        .with_group_items(true)
                        .with_page_size(PAGE_SIZE);
                    let tags = todo_list.get_tags();
                    for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
                        let tag = &chunk[0].tag;
                        let group =
                            format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
                        edit_select.add_items(chunk, Some(&group));
                    }
                    match edit_select.prompt() {
                        Ok(mut task_result) => {
                            let multi_selected = task_result.selected.len() > 1;
                            let mut field_select =
                                Select::<String>::new("Select a field to edit", |i| i.clone())
                                    .with_disabled_items(if multi_selected {
                                        vec![0, 1]
                                    } else {
                                        vec![]
                                    });
                            let mut options = vec![
                                "Description".to_owned(),
                                "Details".to_owned(),
                                "Tag".to_owned(),
                            ];
                            field_select.add_items(options.as_mut_slice(), None);
                            let edited_item: Vec<u64> = task_result
                                .selected
                                .iter()
                                .map(|(_, item)| item.timestamp)
                                .collect();
                            let mut edited_item_clone = vec![];
                            let old_tag;
                            let mut new_tag = None;
                            let old_desc;
                            let mut new_desc = None;
                            let old_detail;
                            let mut new_detail = None;
                            match field_select.prompt() {
                                Ok(result) => match result.index() {
                                    0 => {
                                        let selected = task_result.data_mut();
                                        old_desc = selected.description.clone();
                                        old_tag = selected.tag.clone();
                                        old_detail = selected.detail.clone();
                                        match Text::new("Enter new Task description")
                                            .with_default(&selected.description)
                                            .with_initial_value(&selected.description)
                                            .prompt()
                                        {
                                            Ok(new) => {
                                                new_desc = Some(new.clone());
                                                selected.description = new;
                                                edited_item_clone.push(selected.clone());
                                            }
                                            Err(err) => process::exit(handle_ui_error(
                                                err,
                                                "No Tasks has been edited!",
                                            )),
                                        }
                                    }
                                    1 => {
                                        let selected = task_result.data_mut();
                                        old_desc = selected.description.clone();
                                        old_tag = selected.tag.clone();
                                        old_detail = selected.detail.clone();
                                        match Text::new("Enter new Task Detail")
                                            .with_default(&selected.detail)
                                            .with_initial_value(&selected.detail)
                                            .prompt()
                                        {
                                            Ok(new) => {
                                                new_detail = Some(new.clone());
                                                selected.detail = new;
                                                edited_item_clone.push(selected.clone());
                                            }
                                            Err(err) => process::exit(handle_ui_error(
                                                err,
                                                "No Tasks has been edited!",
                                            )),
                                        }
                                    }
                                    2 => {
                                        match Text::new(if multi_selected {
                                            "Enter new Tag for Tasks"
                                        } else {
                                            "Enter new Tag for Task"
                                        })
                                        .with_auto_completion(tags)
                                        .with_suggest_on_empty(true)
                                        .prompt()
                                        {
                                            Ok(new) => {
                                                {
                                                    let selected = task_result.data();
                                                    old_desc = selected.description.clone();
                                                    old_tag = selected.tag.clone();
                                                    old_detail = selected.detail.clone();
                                                }

                                                if multi_selected {
                                                    for (_, item) in task_result.selected {
                                                        item.tag = new.clone();
                                                        edited_item_clone.push(item.clone());
                                                    }
                                                } else {
                                                    let selected = task_result.data_mut();
                                                    selected.tag = new.clone();
                                                }
                                                new_tag = Some(new.clone());
                                            }
                                            Err(err) => process::exit(handle_ui_error(
                                                err,
                                                "No Tasks has been edited!",
                                            )),
                                        }
                                    }
                                    _ => break,
                                },
                                Err(ui::UIError::Canceled) => {
                                    ui::clean_line();
                                    continue;
                                }
                                Err(err) => {
                                    process::exit(handle_ui_error(err, "No Tasks has been edited!"))
                                }
                            }
                            todo_list.sort();
                            if let Err(err) = todo_list.save(if multi_selected {
                                TodoAction::EditBulk {
                                    edited: edited_item_clone,
                                    old_tag,
                                    new_tag: new_tag.unwrap(),
                                }
                            } else {
                                TodoAction::EditSingular {
                                    edited: edited_item_clone[0].clone(),
                                    old_desc,
                                    new_desc,
                                    old_detail,
                                    new_detail,
                                    old_tag,
                                    new_tag,
                                }
                            }) {
                                process::exit(handle_save_error(err))
                            }
                            todo_list.print_highlight(
                                &edited_item,
                                Some(Color::Yellow),
                                Some(Attributes::default().with(Attribute::Bold)),
                            );
                            break;
                        }
                        Err(err) => {
                            process::exit(handle_ui_error(err, "No Tasks has been edited!"))
                        }
                    }
                }
            }
        }
        Command::Remove => {
            if let Ok(mut todo_list) = todo_list {
                let mut remove_multi_selection =
                    Select::<TodoItem>::new("Select a Task or Multiple Tasks for removal", |i| {
                        let status = if i.done { DONE_PREFIX } else { PENDING_PREFIX };
                        let detail_status = if i.detail.is_empty() {
                            "".reset()
                        } else {
                            " [+]".cyan().bold()
                        };
                        format!(" {} {}{}", status, i.description, detail_status)
                    })
                    .with_multi_selection(true)
                    .with_hybrid(true)
                    .with_group_items(true)
                    .with_finished_formatter(|item| item.description.clone())
                    .with_page_size(PAGE_SIZE)
                    .with_selected_color(Color::Red);
                for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
                    let tag = &chunk[0].tag;
                    let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
                    remove_multi_selection.add_items(chunk, Some(&group));
                }
                let mut to_remove: Vec<u64>;
                let mut remove_item = vec![];
                match remove_multi_selection.prompt() {
                    Ok(result) => {
                        to_remove = result
                            .selected
                            .into_iter()
                            .map(|(_, item)| {
                                remove_item.push(item.clone());
                                item.timestamp
                            })
                            .collect();
                    }
                    Err(err) => process::exit(handle_ui_error(err, "No Tasks has been removed!")),
                }
                todo_list.print_highlight(
                    &to_remove,
                    Some(Color::Red),
                    Some(
                        Attributes::default()
                            .with(Attribute::Bold)
                            .with(Attribute::CrossedOut),
                    ),
                );
                todo_list.remove(&mut to_remove);
                if let Err(err) = todo_list.save(TodoAction::Remove(remove_item)) {
                    process::exit(handle_save_error(err))
                }
            }
        }
        Command::Config => {
            if let Ok(mut todo_list) = todo_list {
                let old_config = todo_list.config.clone();
                loop {
                    let mut option_select =
                        Select::<String>::new("Selected a config option to change", |i| i.clone())
                            .with_finished_formatter(|i| i.clone().split_off(9));
                    let mut options = vec![
                        format!(
                            "{:<8} Show Creation/Completion Time of Tasks",
                            format!("[{}]", todo_list.config.show_date)
                        ),
                        format!(
                            "{:<8} Show Process of Task Groups",
                            format!("[{}]", todo_list.config.show_progress)
                        ),
                        format!(
                            "{:<8} Show Completed Tasks in Hour Range",
                            format!("[{}]", todo_list.config.show_completed_range.unwrap_or(0))
                        ),
                    ];
                    option_select.add_items(&mut options, None);
                    match option_select.prompt() {
                        Ok(result) => match result.index() {
                            0 => {
                                let confirm = Confirm::new("Show Creation Time of Tasks?")
                                    .with_default(todo_list.config.show_date)
                                    .prompt();
                                match confirm {
                                    Ok(yes) => {
                                        todo_list.config.show_date = yes;
                                        break;
                                    }
                                    Err(ui::UIError::Canceled) => {
                                        ui::clean_line();
                                        continue;
                                    }
                                    Err(err) => process::exit(handle_ui_error(
                                        err,
                                        "Config has been left unchanged!",
                                    )),
                                }
                            }
                            1 => {
                                let confirm = Confirm::new("Show Progress of Task Groups?")
                                    .with_default(todo_list.config.show_progress)
                                    .prompt();
                                match confirm {
                                    Ok(yes) => {
                                        todo_list.config.show_progress = yes;
                                        break;
                                    }
                                    Err(ui::UIError::Canceled) => {
                                        ui::clean_line();
                                        continue;
                                    }
                                    Err(err) => process::exit(handle_ui_error(
                                        err,
                                        "Config has been left unchanged!",
                                    )),
                                }
                            }
                            2 => {
                                let range =
                                    Text::new("Enter Hour Range for Completed Tasks to be shown")
                                        .with_hint("'-1' means Inf")
                                        .with_validation(|input| {
                                            let parsed: Result<i64, _> = input.parse();
                                            match parsed {
                                        Ok(range) => {
                                            if range > 999999 {
                                                (
                                                    false,
                                                    Some(
                                                        "Hour Range shall not be over 6 digits!"
                                                            .to_owned(),
                                                    ),
                                                )
                                            } else {
                                                (true, None)
                                            }
                                        }
                                        Err(_) => (
                                            false,
                                            Some(
                                                "Hour Range must be a valid positive Integer!"
                                                    .to_owned(),
                                            ),
                                        ),
                                    }
                                        })
                                        .with_initial_value(
                                            &todo_list
                                                .config
                                                .show_completed_range
                                                .unwrap_or(0)
                                                .to_string(),
                                        )
                                        .prompt();
                                match range {
                                    Ok(range) => {
                                        todo_list.config.show_completed_range =
                                            match range.parse::<i64>().unwrap_or(0) {
                                                -1 => None,
                                                x => Some(x as u64),
                                            }
                                    }
                                    Err(ui::UIError::Canceled) => {
                                        ui::clean_line();
                                        continue;
                                    }
                                    Err(err) => process::exit(handle_ui_error(
                                        err,
                                        "Config has been left unchanged!",
                                    )),
                                }
                                break;
                            }
                            _ => {}
                        },
                        Err(err) => {
                            process::exit(handle_ui_error(err, "Config has been left unchanged!"))
                        }
                    }
                }
                if let Err(err) = todo_list.save(TodoAction::Config {
                    old: old_config,
                    new: todo_list.config.clone(),
                }) {
                    process::exit(handle_save_error(err))
                }
            }
        }
        Command::Detail => {
            if let Ok(mut todo_list) = todo_list {
                let mut task_select =
                    Select::<TodoItem>::new("Select a Task to see its detail", |i| {
                        let status = if i.detail.is_empty() {
                            "".reset()
                        } else {
                            " [+]".cyan().bold()
                        };
                        format!("{}{}", i.description, status)
                    })
                    .with_finished_formatter(|i| i.description.clone())
                    .with_multi_selection(true)
                    .with_hybrid(true)
                    .with_group_items(true)
                    .with_page_size(PAGE_SIZE);
                for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
                    let tag = &chunk[0].tag;
                    let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
                    task_select.add_items(chunk, Some(&group));
                }
                match task_select.prompt() {
                    Ok(result) => {
                        for (_, item) in result.selected {
                            let created_on = {
                                if let Some(date) =
                                    DateTime::from_timestamp_millis(item.timestamp as i64)
                                {
                                    let local_date: DateTime<Local> = DateTime::from(date);
                                    local_date.format("%d-%m-%Y %H:%M").to_string()
                                } else {
                                    "!err!".to_string()
                                }
                            };
                            let completed_on = {
                                if let Some(timestamp) = item.completed_timestamp {
                                    if let Some(date) =
                                        DateTime::from_timestamp_millis(timestamp as i64)
                                    {
                                        let local_date: DateTime<Local> = DateTime::from(date);
                                        local_date
                                            .format("%d-%m-%Y %H:%M")
                                            .to_string()
                                            .green()
                                            .bold()
                                    } else {
                                        "!err!".to_string().reset()
                                    }
                                } else {
                                    "Not completed".to_string().dim().green().italic()
                                }
                            };
                            let detail = if item.detail.is_empty() {
                                "Not provided".to_string().dim().italic()
                            } else {
                                textwrap::fill(
                                    &item.detail,
                                    textwrap::Options::new(80)
                                        .subsequent_indent(" ".repeat(16).as_str()),
                                )
                                .italic()
                                .bold()
                            };
                            println!();
                            println!(
                                "{}{:>15}{} {}",
                                Attribute::Dim,
                                "Description:",
                                Attribute::Reset,
                                item.description.clone().bold()
                            );
                            println!(
                                "{}{:>15}{} {}",
                                Attribute::Dim,
                                "Tag:",
                                Attribute::Reset,
                                item.tag.clone().cyan().bold()
                            );
                            println!(
                                "{}{:>15}{} {}",
                                Attribute::Dim,
                                "Created on:",
                                Attribute::Reset,
                                created_on.italic()
                            );
                            println!(
                                "{}{:>15}{} {}",
                                Attribute::Dim,
                                "Completed on:",
                                Attribute::Reset,
                                completed_on
                            );
                            println!(
                                "{}{:>15}{} {}",
                                Attribute::Dim,
                                "Detail:",
                                Attribute::Reset,
                                detail
                            );
                            println!()
                        }
                    }
                    Err(err) => {
                        process::exit(handle_ui_error(err, "Config has been left unchanged!"))
                    }
                }
            }
        }
        Command::Help => {
            todo::print_help();
        }
        Command::Unknown(cmd) => {
            eprintln!("Error: No such command `{cmd}`\nTry --help to see the available commands",);
        }
        Command::Journal => {
            if let Ok(mut todo_list) = todo_list {
                let base_path = Path::new(&todo_list.path);
                let journal_file =
                    base_path
                        .join(todo::JOURNAL)
                        .into_string()
                        .unwrap_or_else(|_| {
                            eprintln!("Can't fetch journal.");
                            process::exit(1);
                        });
                let file = match File::open(journal_file) {
                    Ok(file) => file,
                    Err(err) => match err.kind() {
                        NotFound => {
                            eprintln!("Journal is empty! Try doing something first");
                            process::exit(1);
                        }
                        _ => {
                            eprintln!("Can't open journal. {}", err.to_string());
                            process::exit(1);
                        }
                    },
                };

                let reader = BufReader::new(file);

                let mut journals = vec![];

                for line in reader.lines() {
                    if let Ok(json) = line {
                        let journal =
                            serde_json::from_str::<TodoJournal>(&json).unwrap_or_else(|e| {
                                eprintln!("Can't parse journal. {e}");
                                process::exit(1);
                            });
                        journals.push(journal);
                    }
                }

                journals.reverse();

                if journals.is_empty() {
                    eprintln!("Journal is empty! Try doing something first");
                    process::exit(1);
                }

                let mut initial_journal_selected = 0;

                loop {
                    let mut journal_select =
                        Select::<TodoJournal>::new("Select a Journal to inspect", |j| {
                            let mut time_string = "!err!".to_string();
                            if let Some(date_time) =
                                DateTime::from_timestamp_millis(j.timestamp as i64)
                            {
                                let local: DateTime<Local> = DateTime::from(date_time);
                                time_string = local.format("%d-%m-%Y %H:%M").to_string();
                            };
                            let action_string;
                            match &j.action {
                                TodoAction::Add(item) => {
                                    action_string =
                                        format!("Added [{}]", truncate(&item.description, 50));
                                }
                                TodoAction::Complete(completed, uncompleted) => {
                                    action_string = format!(
                                        "Completed {} Tasks, Uncompleted {} Tasks",
                                        completed.len(),
                                        uncompleted.len()
                                    );
                                }
                                TodoAction::Remove(items) => {
                                    action_string = format!("Removed {} Tasks", items.len());
                                }
                                TodoAction::EditBulk {
                                    edited,
                                    old_tag,
                                    new_tag,
                                } => {
                                    action_string = format!(
                                        "Edited Tags of {} Tasks from [{}] to [{}]",
                                        edited.len(),
                                        old_tag,
                                        new_tag
                                    );
                                }
                                TodoAction::EditSingular {
                                    edited,
                                    old_desc,
                                    new_desc,
                                    old_detail,
                                    new_detail,
                                    old_tag,
                                    new_tag,
                                } => {
                                    let description_changed = match new_desc {
                                        Some(new_desc) => {
                                            format!(
                                                "Description: [{}] -> [{}]",
                                                truncate(old_desc, 20),
                                                truncate(new_desc, 20)
                                            )
                                        }
                                        None => String::new(),
                                    };
                                    let detail_changed = match new_detail {
                                        Some(new_detail) => {
                                            format!(
                                                "Detail: [{}] -> [{}]",
                                                truncate(old_detail, 20),
                                                truncate(new_detail, 20)
                                            )
                                        }
                                        None => String::new(),
                                    };
                                    let tag_changed = match new_tag {
                                        Some(new_tag) => {
                                            format!(
                                                "Tag: [{}] -> [{}]",
                                                truncate(old_tag, 20),
                                                truncate(new_tag, 20)
                                            )
                                        }
                                        None => String::new(),
                                    };
                                    action_string = format!(
                                        "Edited Task [{}]. {}{}{}",
                                        truncate(&edited.description, 20),
                                        description_changed,
                                        detail_changed,
                                        tag_changed
                                    );
                                }
                                TodoAction::Config { old, new } => {
                                    let mut changed = String::new();
                                    if old.show_date != new.show_date {
                                        changed = format!(
                                            "Show date: {} -> {}",
                                            old.show_date, new.show_date
                                        )
                                    } else if old.show_progress != new.show_progress {
                                        changed = format!(
                                            "Show Group Progress: {} -> {}",
                                            old.show_progress, new.show_progress
                                        )
                                    } else if old.show_completed_range != new.show_completed_range {
                                        changed = format!(
                                            "Show Completed Range: {:?} -> {:?}",
                                            old.show_completed_range, new.show_completed_range
                                        )
                                    }
                                    action_string = format!("Modified Config. {}", changed);
                                }
                                TodoAction::Revert(timestamp) => {
                                    action_string = format!("Reverted back to {}", timestamp);
                                }
                            }

                            format!("{} [{}] {}", j.timestamp, time_string, action_string)
                        })
                        .with_page_size(PAGE_SIZE.min(journals.len()))
                        .with_initial_selection(vec![initial_journal_selected]);
                    journal_select.add_items(&mut journals, None);
                    match journal_select.prompt() {
                        Ok(mut result) => {
                            initial_journal_selected = result.selected[0].0;
                            let selected = result.data_mut();
                            if let Ok(snapshot_file) = base_path
                                .join(HISTORY_DIR)
                                .join(format!("{}.json", selected.timestamp))
                                .into_string()
                            {
                                match fs::read_to_string(&snapshot_file) {
                                    Ok(content) => {
                                        match serde_json::from_str::<TodoList>(&content) {
                                            Ok(mut snapshot) => {
                                                let original_config = snapshot.config.clone();
                                                snapshot.config.show_completed_range = None;
                                                let lines = match &selected.action {
                                                    TodoAction::Add(added) => snapshot
                                                        .print_highlight(
                                                            &[added.timestamp],
                                                            Some(Color::Yellow),
                                                            Some(
                                                                Attributes::default()
                                                                    .with(Attribute::Bold),
                                                            ),
                                                        ),
                                                    TodoAction::Complete(
                                                        completed,
                                                        uncompleted,
                                                    ) => {
                                                        completed
                                                            .clone()
                                                            .append(&mut uncompleted.clone());
                                                        snapshot.print_highlight(
                                                            &completed
                                                                .iter()
                                                                .map(|item| item.timestamp)
                                                                .collect::<Vec<_>>(),
                                                            Some(Color::Yellow),
                                                            Some(
                                                                Attributes::default()
                                                                    .with(Attribute::Bold),
                                                            ),
                                                        )
                                                    }
                                                    TodoAction::Remove(removed) => {
                                                        for item in removed {
                                                            snapshot.items.push(item.clone());
                                                        }
                                                        snapshot.sort();
                                                        snapshot.print_highlight(
                                                            &removed
                                                                .iter()
                                                                .map(|item| item.timestamp)
                                                                .collect::<Vec<_>>(),
                                                            Some(Color::Red),
                                                            Some(
                                                                Attributes::default()
                                                                    .with(Attribute::Bold)
                                                                    .with(Attribute::CrossedOut),
                                                            ),
                                                        )
                                                    }
                                                    TodoAction::EditBulk {
                                                        edited,
                                                        old_tag: _,
                                                        new_tag: _,
                                                    } => snapshot.print_highlight(
                                                        &edited
                                                            .iter()
                                                            .map(|item| item.timestamp)
                                                            .collect::<Vec<_>>(),
                                                        Some(Color::Yellow),
                                                        Some(
                                                            Attributes::default()
                                                                .with(Attribute::Bold),
                                                        ),
                                                    ),
                                                    TodoAction::EditSingular {
                                                        edited,
                                                        old_desc: _,
                                                        new_desc: _,
                                                        old_detail: _,
                                                        new_detail: _,
                                                        old_tag: _,
                                                        new_tag: _,
                                                    } => snapshot.print_highlight(
                                                        &[edited.timestamp],
                                                        Some(Color::Yellow),
                                                        Some(
                                                            Attributes::default()
                                                                .with(Attribute::Bold),
                                                        ),
                                                    ),
                                                    TodoAction::Config { old: _, new: _ }
                                                    | TodoAction::Revert(_) => snapshot.print(),
                                                };
                                                let option_confirmation =
                                                    Confirm::new("Revert to this version?")
                                                        .prompt();
                                                match option_confirmation {
                                                    Ok(true) => {
                                                        if initial_journal_selected == 0 {
                                                            break
                                                        }
                                                        snapshot.config = original_config;
                                                        todo_list = snapshot;
                                                        if let Err(err) = todo_list.save(
                                                            TodoAction::Revert(selected.timestamp),
                                                        ) {
                                                            process::exit(handle_save_error(err))
                                                        }
                                                    }
                                                    _ => {
                                                        clean_lines((lines + 1) as u16);
                                                        continue;
                                                    }
                                                }
                                                break;
                                            }
                                            Err(e) => {
                                                clean_line();
                                                eprintln!(
                                                    "\nFailed to read snapshot {}. {}",
                                                    selected.timestamp,
                                                    e.to_string()
                                                );
                                                continue;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        clean_line();
                                        eprintln!(
                                            "\nFailed to fetch snapshot {}. {}",
                                            selected.timestamp,
                                            e.to_string()
                                        );
                                        continue;
                                    }
                                }
                            } else {
                                clean_line();
                                continue;
                            }
                        }
                        Err(err) => {
                            process::exit(handle_ui_error(err, "Config has been left unchanged!"))
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    let todo_list = match find_todo_list() {
        Ok(todo_list) => Ok(todo_list),
        Err(err) => {
            match &err {
                TodoFindError::NotFound => {
                    eprintln!(".todo not found!");
                }
                TodoFindError::Invalid(_) => {
                    eprintln!(".todo found, but it is invalid or corrupted!");
                }
                TodoFindError::Other(e) => {
                    eprintln!("Failed to find .todo: {}", e);
                }
            }
            Err(err)
        }
    };

    run(env::args().collect(), todo_list)
}
