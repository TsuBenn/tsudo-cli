use std::{env, process};

mod todo;

use crossterm::style::{Attribute, Attributes, Color};

use tsudo::ui::{self, Confirm, Select, Text};

use crate::todo::*;

const PAGE_SIZE: usize = 20;
const DONE_PREFIX: &'static str = "Done";
const PENDING_PREFIX: &'static str = " -- ";

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
            eprintln!("Problem with saving to 'todo.json'! {e}");
            eprintln!("No change has been made!");
            1
        }
    }
}

fn run(args: Vec<String>, todo_list: Result<TodoList, TodoFindError>) {
    match get_command(args) {
        Command::List => {
            if let Ok(todo_list) = todo_list {
                todo_list.print();
            }
        }
        Command::New(mut description) => {
            let mut new_todo_list;
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
                        Ok(false) => process::exit(handle_ui_error(
                            ui::UIError::Canceled,
                            "No 'todo.json' has been created!",
                        )),
                        Err(err) => {
                            process::exit(handle_ui_error(err, "No 'todo.json' has been created!"))
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
            let new_item_timestamp = new_todo_list.add(description, tag);
            if let Err(err) = new_todo_list.save() {
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
                    Ok(result) => {
                        let mut changed_timestamps: Vec<u64> = result
                            .selected
                            .iter()
                            .filter_map(|(_, item)| {
                                if !old_completed_timestamps.contains(&item.timestamp) {
                                    Some(item.timestamp)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        changed_timestamps.append(
                            &mut result
                                .unselected
                                .iter()
                                .filter_map(|(_, item)| {
                                    if old_completed_timestamps.contains(&item.timestamp) {
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
                        if let Err(err) = todo_list.save() {
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
                let mut edit_select =
                    Select::<TodoItem>::new("Select a Task or multiple Tasks to edit", |i| {
                        let status = if i.done { DONE_PREFIX } else { PENDING_PREFIX };
                        format!(" {}  {}", status, i.description)
                    })
                    .with_multi_selection(true)
                    .with_hybrid(true)
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
                        let edited_item: Vec<u64> = task_result
                            .selected
                            .iter()
                            .map(|(_, item)| item.timestamp)
                            .collect();
                        match field_select.prompt() {
                            Ok(result) => match result.index() {
                                0 => {
                                    let selected = task_result.data_mut();
                                    match Text::new("Enter new Task description")
                                        .with_default(&selected.description)
                                        .with_initial_value(&selected.description)
                                        .prompt()
                                    {
                                        Ok(new_description) => {
                                            selected.description = new_description;
                                        }
                                        Err(err) => process::exit(handle_ui_error(
                                            err,
                                            "No Tasks has been edited!",
                                        )),
                                    }
                                }
                                1 => {
                                    match Text::new(if multi_selected {"Enter new Tasks Tag"} else {"Enter new Task Tag"})
                                        .with_auto_completion(tags)
                                        .with_suggest_on_empty(true)
                                        .prompt()
                                    {
                                        Ok(new_tag) => {
                                            for (_, item) in task_result.selected {
                                                item.tag = new_tag.clone();
                                            }
                                        }
                                        Err(err) => process::exit(handle_ui_error(
                                            err,
                                            "No Tasks has been edited!",
                                        )),
                                    }
                                }
                                _ => {}
                            },
                            Err(err) => {
                                process::exit(handle_ui_error(err, "No Tasks has been edited!"))
                            }
                        }
                        todo_list.sort();
                        if let Err(err) = todo_list.save() {
                            process::exit(handle_save_error(err))
                        }
                        todo_list.print_highlight(
                            &edited_item,
                            Some(Color::Yellow),
                            Some(Attributes::default().with(Attribute::Bold)),
                        );
                    }
                    Err(err) => process::exit(handle_ui_error(err, "No Tasks has been edited!")),
                }
            }
        }
        Command::Remove => {
            if let Ok(mut todo_list) = todo_list {
                let mut remove_multi_selection =
                    Select::<TodoItem>::new("Select Tasks for removal", |i| {
                        let status = if i.done { DONE_PREFIX } else { PENDING_PREFIX };
                        format!(" {}  {}", status, i.description)
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
                let mut to_remove: Vec<u64>;
                match remove_multi_selection.prompt() {
                    Ok(result) => {
                        to_remove = result
                            .selected
                            .iter()
                            .map(|(_, item)| item.timestamp)
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
                if let Err(err) = todo_list.save() {
                    process::exit(handle_save_error(err))
                }
            }
        }
        Command::Config => {
            if let Ok(mut todo_list) = todo_list {
                loop {
                    let mut option_select =
                        Select::<String>::new("Selected a config option to change", |i| i.clone());
                    let mut options = vec![
                        "Show Creation Time of Tasks".to_string(),
                        "Show Progress of Task Groups".to_string(),
                    ];
                    option_select.add_items(&mut options, None);
                    match option_select.prompt() {
                        Ok(result) => match result.index() {
                            0 => {
                                let confirm = Confirm::new("Show Creation Time of Tasks?")
                                    .with_default(todo_list.config.show_date_created)
                                    .prompt();
                                match confirm {
                                    Ok(yes) => todo_list.config.show_date_created = yes,
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
                                    Ok(yes) => todo_list.config.show_progress = yes,
                                    Err(err) => process::exit(handle_ui_error(
                                        err,
                                        "Config has been left unchanged!",
                                    )),
                                }
                            }
                            _ => {}
                        },
                        Err(ui::UIError::Canceled) => break,
                        Err(err) => {
                            process::exit(handle_ui_error(err, "Config has been left unchanged!"))
                        }
                    }
                }
                if let Err(err) = todo_list.save() {
                    process::exit(handle_save_error(err))
                }
            }
        }
        _ => {}
    }
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

    run(env::args().collect(), todo_list)
}
