use crate::{cli::util::*, todo::*};
use tsudo::tui::*;

pub fn todo_config(todo_list: Result<TodoList, TodoFindError>) -> Result<(), i32> {
    if let Ok(mut todo_list) = todo_list {
        let old_config = todo_list.config.clone();
        let mut initial_selection = 0;
        loop {
            let mut option_select =
                Select::<String>::new("Selected a config option to change", |i| i.clone())
                    .with_finished_formatter(|i| i.clone().split_off(9))
                    .with_initial_selection(vec![initial_selection]);
            let mut options = vec![
                format!(
                    "{:<8} Show Creation/Completion Time of Tasks",
                    format!("[{}]", todo_list.config.show_date)
                ),
                format!(
                    "{:<8} Show Progress of Task Groups",
                    format!("[{}]", todo_list.config.show_progress)
                ),
                format!(
                    "{:<8} Show Completed Tasks Range",
                    format!(
                        "[{}]",
                        match todo_list.config.show_completed_range {
                            Some(x) => format!("{x}h"),
                            None => "Inf".to_string(),
                        }
                    )
                ),
                format!(
                    "{:<8} Hide Tasks in Fully Completed Groups",
                    format!("[{}]", todo_list.config.hide_completed_groups)
                ),
                format!(
                    "{:<8} Sort tasks by completion",
                    format!("[{}]", todo_list.config.sort_by_completion)
                ),
            ];
            option_select.add_items(&mut options, None);
            match option_select.prompt() {
                Ok(result) => match result.index() {
                    0 => {
                        todo_list.config.show_date = !todo_list.config.show_date;
                        initial_selection = 0;
                        clean_line_once();
                        continue;
                    }
                    1 => {
                        todo_list.config.show_progress = !todo_list.config.show_progress;
                        initial_selection = 1;
                        clean_line_once();
                        continue;
                    }
                    2 => {
                        todo_list.config.show_completed_range =
                            match todo_list.config.show_completed_range {
                                Some(0) => Some(6),
                                Some(6) => Some(12),
                                Some(12) => Some(24),
                                Some(24) => Some(48),
                                Some(48) => None,
                                None | _ => Some(0),
                            };
                        initial_selection = 2;
                        clean_line_once();
                        continue;
                    }
                    3 => {
                        todo_list.config.hide_completed_groups =
                            !todo_list.config.hide_completed_groups;
                        initial_selection = 3;
                        clean_line_once();
                        continue;
                    }
                    4 => {
                        todo_list.config.sort_by_completion = !todo_list.config.sort_by_completion;
                        initial_selection = 4;
                        clean_line_once();
                        continue;
                    }
                    _ => {}
                },
                Err(UIError::Canceled) => break,
                Err(err) => return Err(handle_ui_error(err, "Config has been left unchanged!")),
            }
        }
        if old_config != todo_list.config {
            todo_list
                .save(TodoAction::Config {
                    old: old_config,
                    new: todo_list.config.clone(),
                })
                .map_err(|err| handle_save_error(err))?;
            eprintln!("\n  Config saved!\n")
        } else {
            eprintln!("\n  Config unchanged!\n")
        }
    }

    Ok(())
}
