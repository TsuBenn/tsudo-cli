use crate::{
    cli::{DONE_PREFIX, PAGE_SIZE, PENDING_PREFIX, util::*},
    todo::*,
};
use crossterm::style::{Attribute, Attributes, Color, Stylize};
use tsudo::tui::*;

pub fn todo_edit(todo_list: Result<TodoList, TodoFindError>) -> Result<(), i32> {
    if let Ok(mut todo_list) = todo_list {
        let mut initial_position = 0;
        let mut initial_field_position = 0;
        let mut auto_select_task = None;
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
                .with_initial_position(initial_position)
                .with_hybrid(true)
                .with_collapsed_groups(if todo_list.config.hide_completed_groups {
                    todo_list
                        .get_completed_tags()
                        .into_iter()
                        .map(|s| format!("{}:", s))
                        .collect()
                } else {
                    vec![]
                })
                .with_finished_formatter(|item| item.description.clone())
                .with_group_items(true)
                .with_page_size(PAGE_SIZE);
            if let Some(auto_select) = auto_select_task {
                edit_select = edit_select.with_auto_select(auto_select);
            }
            if todo_list.config.sort_by_completion {
                edit_select = edit_select.with_sorter(|a, b| a.done.cmp(&b.done));
            }
            let tags = todo_list.get_tags();
            for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
                let tag = &chunk[0].tag;
                let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
                edit_select.add_items(chunk, Some(&group));
            }
            let mut task_result = edit_select
                .prompt()
                .map_err(|err| handle_ui_error(err, "No Tasks has been edited!"))?;

            initial_position = task_result.index();
            let multi_selected = task_result.selected.len() > 1;

            let mut field_select = Select::<String>::new("Select a field to edit", |i| i.clone())
                .with_disabled_items(if multi_selected { vec![0, 1] } else { vec![] })
                .with_initial_position(initial_field_position);
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
                        initial_field_position = 0;
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
                            Err(UIError::Canceled) => {
                                auto_select_task = Some(initial_position);
                                clean_lines(2);
                                continue;
                            }
                            Err(err) => {
                                return Err(handle_ui_error(err, "No Tasks has been edited!"));
                            }
                        }
                    }
                    1 => {
                        initial_field_position = 1;
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
                            Err(UIError::Canceled) => {
                                auto_select_task = Some(initial_position);
                                clean_lines(2);
                                continue;
                            }
                            Err(err) => {
                                return Err(handle_ui_error(err, "No Tasks has been edited!"));
                            }
                        }
                    }
                    2 => {
                        initial_field_position = 2;
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
                            Err(UIError::Canceled) => {
                                auto_select_task = Some(initial_position);
                                clean_lines(2);
                                continue;
                            }
                            Err(err) => {
                                return Err(handle_ui_error(err, "No Tasks has been edited!"));
                            }
                        }
                    }
                    _ => break,
                },
                Err(UIError::Canceled) => {
                    clean_line();
                    auto_select_task = None;
                    initial_field_position = 0;
                    continue;
                }
                Err(err) => return Err(handle_ui_error(err, "No Tasks has been edited!")),
            }
            todo_list.sort();
            todo_list
                .save(if multi_selected {
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
                })
                .map_err(|err| handle_save_error(err))?;
            todo_list.items.sort_by_key(|i| i.done);
            todo_list.print_highlight(
                &edited_item,
                Some(Color::Yellow),
                Some(Attributes::default().with(Attribute::Bold)),
            );
            break;
        }
    }

    Ok(())
}
