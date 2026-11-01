use chrono::{DateTime, Local};
use crossterm::style::{Attribute, Stylize};
use tsudo::tui::*;

use crate::{
    cli::{PAGE_SIZE, util::*},
    todo::*,
};

pub fn todo_detail(todo_list: Result<TodoList, TodoFindError>) -> Result<(), i32> {
    if let Ok(mut todo_list) = todo_list {
        let mut task_select = Select::<TodoItem>::new("Select a Task to see its detail", |i| {
            let status = if i.detail.is_empty() {
                "".reset()
            } else {
                " [+]".cyan().bold()
            };
            format!("{}{}", i.description, status)
        })
        .with_finished_formatter(|i| i.description.clone())
        .with_multi_selection(true)
        .with_collapsed_groups(if todo_list.config.hide_completed_groups {
            todo_list
                .get_completed_tags()
                .into_iter()
                .map(|s| format!("{}:", s))
                .collect()
        } else {
            vec![]
        })
        .with_hybrid(true)
        .with_group_items(true)
        .with_page_size(PAGE_SIZE);
        if todo_list.config.sort_by_completion {
            task_select = task_select.with_sorter(|a, b| a.done.cmp(&b.done));
        }
        for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
            task_select.add_items(chunk, Some(&group));
        }
        let result = task_select
            .prompt()
            .map_err(|err| handle_ui_error(err, "Config has been left unchanged!"))?;
        for (_, item) in result.selected {
            let created_on = {
                if let Some(date) = DateTime::from_timestamp_millis(item.timestamp as i64) {
                    let local_date: DateTime<Local> = DateTime::from(date);
                    local_date.format("%d-%m-%Y %H:%M").to_string()
                } else {
                    "!err!".to_string()
                }
            };
            let completed_on = {
                if let Some(timestamp) = item.completed_timestamp {
                    if let Some(date) = DateTime::from_timestamp_millis(timestamp as i64) {
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
                    textwrap::Options::new(80).subsequent_indent(" ".repeat(16).as_str()),
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
    Ok(())
}
