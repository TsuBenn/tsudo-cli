use std::io::{Stdout, stdout};

use crossterm::{
    cursor::{self, MoveToColumn, MoveUp},
    style::{
        Attribute::{self},
        Attributes, Color, Print, ResetColor, SetAttribute, SetAttributes, SetForegroundColor,
        Stylize,
    },
    terminal::{self, Clear},
};

#[derive(Debug)]
pub enum UIError {
    Canceled,
    Terminal(String),
    Other(String),
}

pub fn clean_line() {
    crossterm::execute!(
        stdout(),
        MoveToColumn(0),
        MoveUp(1),
        Clear(terminal::ClearType::FromCursorDown),
        MoveUp(1),
    )
    .unwrap()
}

pub fn clean_line_once() {
    crossterm::execute!(
        stdout(),
        MoveToColumn(0),
        MoveUp(1),
        Clear(terminal::ClearType::FromCursorDown),
    )
    .unwrap()
}

pub fn clean_lines(lines: u16) {
    crossterm::execute!(
        stdout(),
        MoveToColumn(0),
        MoveUp(lines),
        Clear(terminal::ClearType::FromCursorDown),
        MoveUp(1),
    )
    .unwrap()
}

pub fn render_prompt(
    stdout: &mut Stdout,
    message: &str,
    hint: &Option<String>,
    text: &str,
    color: Color,
    attributes: Option<Attributes>,
    new_line: bool,
) -> Result<usize, UIError> {
    let hint = match hint.as_ref() {
        Some(hint) if !hint.is_empty() => format!(" [{}]", hint).cyan().to_string(),
        _ => String::new(),
    };
    let prompt = format!("{} {}{}: ", "?".to_string().green(), &message, &hint);
    crossterm::queue!(
        stdout,
        MoveToColumn(0),
        Clear(terminal::ClearType::FromCursorDown),
        SetAttribute(Attribute::Bold),
        Print(&prompt),
        SetAttribute(Attribute::Reset),
        SetAttributes(match attributes {
            Some(_) => attributes.unwrap(),
            None => Attributes::default(),
        }),
        SetForegroundColor(color),
        Print(format!("{}{}", text, if new_line { "\n" } else { "" })),
        MoveToColumn(0),
        ResetColor,
    )
    .map_err(|e| UIError::Terminal(e.to_string()))?;
    Ok(strip_ansi_escapes::strip_str(&prompt).len())
}

pub fn cleanup_terminal(stdout: &mut Stdout, report_cancel: bool) -> Result<(), UIError> {
    crossterm::execute!(
        stdout,
        cursor::Show,
        cursor::SetCursorStyle::DefaultUserShape
    )
    .map_err(|e| UIError::Terminal(e.to_string()))?;
    crossterm::terminal::disable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;

    if report_cancel {
        return Err(UIError::Canceled);
    }

    Ok(())
}
