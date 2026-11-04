use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use crate::{
    cli::{PAGE_SIZE, util::*},
    todo::*,
};
use chrono::{DateTime, Local};
use crossterm::style::Color;
use tsudo::tui::*;

use crossterm::style::{Attribute, Attributes};

pub fn todo_journal(todo_list: Result<TodoList, TodoFindError>, undo: bool) -> Result<(), i32> {
    if let Ok(mut todo_list) = todo_list {
        let base_path = Path::new(&todo_list.path);
        let journal_file = base_path.join(JOURNAL).into_os_string();
        let file = match File::open(journal_file) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    eprintln!("\n  Journal is empty! Try doing something first\n");
                    return Err(1);
                }
                _ => {
                    eprintln!("\n  Can't open journal. {}\n", err.to_string());
                    return Err(1);
                }
            },
        };

        let reader = BufReader::new(file);

        let mut journals = vec![];

        for line in reader.lines() {
            if let Ok(json) = line {
                let journal = serde_json::from_str::<TodoJournal>(&json).map_err(|e| {
                    eprintln!("\n  Can't parse journal. {e}\n");
                    1
                })?;
                journals.push(journal);
            }
        }

        journals.reverse();

        if journals.is_empty() {
            eprintln!("\n  Journal is empty! Try doing something first\n");
            return Err(1);
        }

        if journals.len() == 1 {
            eprintln!("\n  Can't undo, already at the oldest change!\n");
            return Err(1);
        }

        let mut initial_journal_selected = 0;
        let mut auto_select_journal = if undo { Some(1) } else { None };
        let mut hide_completed_groups = None;
        let mut show_changes = true;
        let mut sort_by_completion = None;
        let mut initial_option_selected = 0;

        loop {
            let mut journal_select = Select::<TodoJournal>::new(
                if undo {
                    "Undo to previous change"
                } else {
                    "Select a Journal to inspect"
                },
                |j| {
                    let mut time_string = "!err!".to_string();
                    if let Some(date_time) = DateTime::from_timestamp_millis(j.timestamp as i64) {
                        let local: DateTime<Local> = DateTime::from(date_time);
                        time_string = local.format("%d-%m-%Y %H:%M").to_string();
                    };
                    let action_string;
                    match &j.action {
                        TodoAction::Add(item) => {
                            action_string = format!("Added [{}]", truncate(&item.description, 50));
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
                                changed =
                                    format!("Show date: {} -> {}", old.show_date, new.show_date)
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
                        TodoAction::Init => {
                            action_string = format!("Journal initialized");
                        }
                    }

                    format!("{} [{}] {}", j.timestamp, time_string, action_string)
                },
            )
            .with_page_size(PAGE_SIZE.min(journals.len()))
            .with_initial_position(initial_journal_selected);
            if let Some(auto_select) = auto_select_journal {
                journal_select = journal_select.with_auto_select(auto_select);
                auto_select_journal = None;
            }
            journal_select.add_items(&mut journals, None);
            let mut result = journal_select
                .prompt()
                .map_err(|err| handle_ui_error(err, "Config has been left unchanged!"))?;
            initial_journal_selected = result.selected[0].0;
            let selected = result.data_mut();
            let snapshot_file = base_path
                .join(HISTORY_DIR)
                .join(format!("{}.json", selected.timestamp))
                .into_os_string();
            match fs::read_to_string(&snapshot_file) {
                Ok(content) => match serde_json::from_str::<TodoList>(&content) {
                    Ok(mut snapshot) => {
                        let original_config = snapshot.config.clone();
                        snapshot.config.show_completed_range = None;
                        if let Some(hide_completed_groups) = hide_completed_groups {
                            snapshot.config.hide_completed_groups = hide_completed_groups;
                        }
                        if let Some(sort_by_completion) = sort_by_completion {
                            snapshot.config.sort_by_completion = sort_by_completion;
                        }
                        let lines = if show_changes {
                            match &selected.action {
                                TodoAction::Add(added) => snapshot.print_highlight(
                                    &[added.timestamp],
                                    Some(Color::Yellow),
                                    Some(Attributes::default().with(Attribute::Bold)),
                                ),
                                TodoAction::Complete(completed, uncompleted) => {
                                    completed.clone().append(&mut uncompleted.clone());
                                    snapshot.print_highlight(
                                        &completed
                                            .iter()
                                            .map(|item| item.timestamp)
                                            .collect::<Vec<_>>(),
                                        Some(Color::Yellow),
                                        Some(Attributes::default().with(Attribute::Bold)),
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
                                    &edited.iter().map(|item| item.timestamp).collect::<Vec<_>>(),
                                    Some(Color::Yellow),
                                    Some(Attributes::default().with(Attribute::Bold)),
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
                                    Some(Attributes::default().with(Attribute::Bold)),
                                ),
                                TodoAction::Config { old: _, new: _ }
                                | TodoAction::Revert(_)
                                | TodoAction::Init => snapshot.print(),
                            }
                        } else {
                            snapshot.print()
                        } + 1;
                        let mut option_select =
                            Select::<String>::new("Revert to this version?", |i| i.clone())
                                .with_initial_position(initial_option_selected);
                        let mut options = vec![
                            "Revert to this version".to_string(),
                            if snapshot.config.hide_completed_groups {
                                "Show completed groups".to_string()
                            } else {
                                "Hide completed groups".to_string()
                            },
                            if show_changes {
                                "Hide changes".to_string()
                            } else {
                                "Show changes".to_string()
                            },
                            if snapshot.config.sort_by_completion {
                                "Stop sorting by Completion".to_string()
                            } else {
                                "Sort by Completion".to_string()
                            },
                        ];
                        option_select.add_items(&mut options, None);
                        if undo {
                            option_select = option_select.with_auto_select(0)
                        }

                        match option_select.prompt() {
                            Ok(result) => match result.index() {
                                0 => {
                                    if initial_journal_selected == 0 {
                                        break;
                                    }
                                    snapshot.config = original_config;
                                    let current_path = todo_list.path;
                                    todo_list = snapshot;
                                    todo_list.path = current_path;
                                    todo_list
                                        .save(TodoAction::Revert(selected.timestamp))
                                        .map_err(|err| handle_save_error(err))?;
                                }
                                1 => {
                                    initial_option_selected = 1;
                                    auto_select_journal = Some(initial_journal_selected);
                                    hide_completed_groups =
                                        Some(!snapshot.config.hide_completed_groups);
                                    clean_lines((lines + 1) as u16);
                                    continue;
                                }
                                2 => {
                                    initial_option_selected = 2;
                                    auto_select_journal = Some(initial_journal_selected);
                                    show_changes = !show_changes;
                                    clean_lines((lines + 1) as u16);
                                    continue;
                                }
                                3 => {
                                    initial_option_selected = 3;
                                    auto_select_journal = Some(initial_journal_selected);
                                    sort_by_completion = Some(!snapshot.config.sort_by_completion);
                                    clean_lines((lines + 1) as u16);
                                    continue;
                                }
                                _ => break,
                            },
                            _ => {
                                initial_option_selected = 0;
                                clean_lines((lines + 1) as u16);
                                continue;
                            }
                        }
                        break;
                    }
                    Err(e) => {
                        clean_line();
                        eprintln!(
                            "\n  Failed to read snapshot {}. {}\n",
                            selected.timestamp,
                            e.to_string()
                        );
                        continue;
                    }
                },
                Err(e) => {
                    clean_line();
                    eprintln!(
                        "\n  Failed to fetch snapshot {}. {}\n",
                        selected.timestamp,
                        e.to_string()
                    );
                    continue;
                }
            }
        }
    }
    Ok(())
}
