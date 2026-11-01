use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::style::{Attribute, Attributes, Color};
use tsudo::tui::*;

use crate::{cli::{PAGE_SIZE, util::*}, todo::*};

pub fn todo_complete(todo_list: Result<TodoList, TodoFindError>) -> Result<(), i32> {
    if let Ok(mut todo_list) = todo_list {
        let old_completed_timestamps: Vec<u64> = todo_list
            .items
            .iter()
            .filter_map(|i| if i.done { Some(i.timestamp) } else { None })
            .collect();
        let mut done_multi_selection =
            Select::<TodoItem>::new("Toggle Tasks completion status", |i| i.description.clone())
                .with_multi_selection(true)
                .with_group_items(true)
                .with_collapsed_groups(if todo_list.config.hide_completed_groups {
                    todo_list
                        .get_completed_tags()
                        .into_iter()
                        .map(|s| format!("{}:", s))
                        .collect()
                } else {
                    vec![]
                })
                .with_page_size(PAGE_SIZE)
                .with_initial_selection(
                    todo_list
                        .items
                        .iter()
                        .enumerate()
                        .filter_map(|(i, item)| if item.done { Some(i) } else { None })
                        .collect(),
                );
        if todo_list.config.sort_by_completion {
            done_multi_selection = done_multi_selection.with_sorter(|a, b| a.done.cmp(&b.done));
        }
        for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
            done_multi_selection.add_items(chunk, Some(&group));
        }
        let mut result = done_multi_selection.prompt().map_err(|err| {
            handle_ui_error(err, "No completion status of Tasks has been changed!")
        })?;
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
                            .as_millis() as u64,
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
        todo_list.save(TodoAction::Complete(completed_item, uncompleted_item)).map_err(|err| handle_save_error(err))?;

        todo_list.items.sort_by_key(|i| i.done);
        todo_list.print_highlight(
            &changed_timestamps,
            Some(Color::Yellow),
            Some(Attributes::default().with(Attribute::Bold)),
        );
    }

    Ok(())
}
