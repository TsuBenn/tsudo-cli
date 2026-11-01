use crate::{
    cli::{DONE_PREFIX, PAGE_SIZE, PENDING_PREFIX, util::*}, todo::*,
};
use crossterm::style::{Color, Stylize};
use tsudo::tui::*;

use crossterm::style::{Attribute, Attributes};

pub fn todo_remove(todo_list: Result<TodoList, TodoFindError>) -> Result<(), i32> {
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
            .with_page_size(PAGE_SIZE)
            .with_selected_color(Color::Red);
        if todo_list.config.sort_by_completion {
            remove_multi_selection = remove_multi_selection.with_sorter(|a, b| a.done.cmp(&b.done));
        }
        for chunk in todo_list.items.chunk_by_mut(|a, b| a.tag == b.tag) {
            let tag = &chunk[0].tag;
            let group = format!("{}:", if tag.is_empty() { "__untagged__" } else { tag });
            remove_multi_selection.add_items(chunk, Some(&group));
        }
        let mut to_remove: Vec<u64>;
        let mut remove_item = vec![];
        let result =  remove_multi_selection.prompt().map_err(|err| handle_ui_error(err, "No Tasks has been removed!"))?;
        to_remove = result
            .selected
            .into_iter()
            .map(|(_, item)| {
                remove_item.push(item.clone());
                item.timestamp
            })
            .collect();
        let mut todo_list_clone = todo_list.clone();
        todo_list_clone.items.sort_by_key(|i| i.done);
        todo_list.remove(&mut to_remove);
        todo_list.save(TodoAction::Remove(remove_item)).map_err(|err| handle_save_error(err))?;
        todo_list_clone.print_highlight(
            &to_remove,
            Some(Color::Red),
            Some(
                Attributes::default()
                    .with(Attribute::Bold)
                    .with(Attribute::CrossedOut),
            ),
        );
    }
    Ok(())
}
