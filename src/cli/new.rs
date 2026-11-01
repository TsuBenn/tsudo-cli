use crossterm::style::{Attribute, Attributes, Color};
use tsudo::tui::*;
use crate::todo::*;

use crate::{
    cli::util::{handle_save_error, handle_ui_error}, todo::{TodoFindError, TodoList},
};

pub fn todo_new(todo_list: Result<TodoList, TodoFindError>, description: String) -> Result<(), i32> {
    let mut description = description;
    let mut new_todo_list;
    match todo_list {
        Ok(todo_list) => new_todo_list = todo_list,
        Err(TodoFindError::Invalid(file_path)) => {
            let selection = Confirm::new("Override current .todo?")
                .with_default(true)
                .prompt()
                .map_err(|_| handle_ui_error(UIError::Canceled, "No change has been made!"))?;
            if selection {
                new_todo_list = TodoList::new();
                new_todo_list.path = file_path
            } else {
                return Err(handle_ui_error(UIError::Canceled, "No change has been made!"));
            }
        }
        Err(_) => {
            let selection = Confirm::new("Create new .todo?")
                .with_default(true)
                .prompt()
                .map_err(|_| {
                    handle_ui_error(UIError::Canceled, "No 'todo.json' has been created!")
                })?;
            if selection {
                new_todo_list = TodoList::new();
            } else {
                return Err(handle_ui_error(UIError::Canceled, "No change has been made!"));
            }
        }
    }
    if description.is_empty() {
        let new_description = Text::new("Enter a description for the Task")
            .prompt()
            .map_err(|err| handle_ui_error(err, "No Task has been added!"))?;
        description = new_description;
    }
    let tag = Text::new("Enter a Tag for the Task")
        .with_default("")
        .with_auto_completion(new_todo_list.get_tags())
        .with_suggest_on_empty(true)
        .prompt()
        .map_err(|err| handle_ui_error(err, "No Task has been added!"))?;
    let new_item_timestamp = new_todo_list.add(description.clone(), tag.clone());
    new_todo_list.save(TodoAction::Add(TodoItem::new(description, tag))).map_err(|err| handle_save_error(err))?;
    new_todo_list.print_highlight(
        &[new_item_timestamp],
        Some(Color::Yellow),
        Some(Attributes::default().with(Attribute::Bold)),
    );

    Ok(())
}
