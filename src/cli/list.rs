use crate::todo::{TodoFindError, TodoList};

pub fn todo_list(todo_list: Result<TodoList, TodoFindError>, all: bool, detail: bool, time: bool, pending: bool) {
    if let Ok(mut todo_list) = todo_list {
        if all {
            todo_list.config.show_completed_range = None;
            todo_list.config.hide_completed_groups = false
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
        todo_list.items.sort_by_key(|i| i.done);
        todo_list.print();
    }
}
