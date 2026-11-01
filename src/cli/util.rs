use tsudo::tui::*;
use crate::todo::*;

pub fn handle_ui_error(err: UIError, cancel_message: &str) -> i32 {
    match err {
        UIError::Canceled => {
            eprintln!("\n  {}\n", cancel_message);
            0
        }
        UIError::Terminal(err) => {
            eprintln!("\n  Terminal error: {err}\n");
            1
        }
        UIError::Other(err) => {
            eprintln!("\n  Error: {err}\n");
            1
        }
    }
}

pub fn handle_save_error(err: TodoListError) -> i32 {
    match err {
        TodoListError::SaveError(e) => {
            eprintln!("\n  Problem with saving to .todo! {e}\n");
            eprintln!("\n  No change has been made!\n");
            1
        }
    }
}

pub fn truncate(text: &str, max: usize) -> String {
    if text.len() > max {
        let mut text: String = text.chars().take(max - 1).collect();
        text.push('…');
        return text;
    }
    text.to_string()
}
