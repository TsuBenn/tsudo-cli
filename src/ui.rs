use std::{
    io::{Stdout, Write, stdout},
    os::raw::c_ushort,
};

use crossterm::{
    cursor::{self, MoveDown, MoveToColumn, MoveToNextLine, MoveUp}, event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode}, style::{Attribute::{self, Bold}, Color, Print, ResetColor, SetAttribute, SetAttributes, SetBackgroundColor, SetForegroundColor, Stylize}, terminal::{self, Clear, ScrollDown, ScrollUp},
};

#[derive(Debug)]
pub enum UIError {
    Canceled,
    Terminal(String),
    Other(String),
}

pub struct Confirm<'a> {
    message: &'a str,
    default: bool,
}

impl<'a> Confirm<'a> {
    pub fn new(message: &'a str) -> Self {
        Self {
            message,
            default: true,
        }
    }

    pub fn with_default(mut self, default: bool) -> Self {
        self.default = default;
        self
    }

    pub fn prompt(&self) -> Result<bool, UIError> {
        let mut stdout = stdout();
        let mut selection = self.default;

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout, cursor::Hide).map_err(|e| UIError::Terminal(e.to_string()))?;

        let options = if self.default { "[Y/n]" } else { "[y/N]" };

        loop {
            crossterm::execute!(
                stdout,
                MoveToColumn(0),
                Clear(terminal::ClearType::CurrentLine),
                Print(format!("{} {}:", self.message, options)),
            )
            .map_err(|e| UIError::Terminal(e.to_string()))?;

            if let Event::Key(key_event) =
                event::read().map_err(|e| UIError::Terminal(e.to_string()))?
            {
                if key_event.kind != KeyEventKind::Press {
                    continue;
                }

                match key_event.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        selection = true;
                        break;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        selection = false;
                        break;
                    }
                    KeyCode::Esc => {
                        Confirm::render_finish(
                            &mut stdout,
                            self.message,
                            options,
                            "<canceled>",
                            Color::Red,
                        )?;
                        crossterm::execute!(stdout, cursor::Show)
                            .map_err(|e| UIError::Terminal(e.to_string()))?;
                        crossterm::terminal::disable_raw_mode()
                            .map_err(|e| UIError::Terminal(e.to_string()))?;
                        return Err(UIError::Canceled);
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }

        Confirm::render_finish(
            &mut stdout,
            self.message,
            options,
            if selection { "Yes" } else { "No" },
            Color::Green,
        )?;

        crossterm::execute!(stdout, cursor::Show).map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::terminal::disable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;

        Ok(selection)
    }

    fn render_finish(
        stdout: &mut Stdout,
        message: &'a str,
        options: &str,
        answer: &str,
        color: Color,
    ) -> Result<(), UIError> {
        crossterm::execute!(
            stdout,
            MoveToColumn(0),
            Clear(terminal::ClearType::CurrentLine),
            Print(format!("{} {}: ", message, options)),
            SetForegroundColor(color),
            Print(format!("{}\n", answer)),
            MoveToColumn(0),
            ResetColor
        )
        .map_err(|e| UIError::Terminal(e.to_string()))
    }
}

pub struct Text<'a> {
    message: &'a str,
    default: Option<String>,
    hint: Option<String>,
    initial_value: String,
    suggestions: Option<Vec<String>>,
    max_suggestions: Option<usize>,
    suggest_on_empty: bool,
}

impl<'a> Text<'a> {
    pub fn new(message: &'a str) -> Self {
        Self {
            message,
            default: None,
            hint: None,
            initial_value: String::new(),
            suggestions: None,
            max_suggestions: Some(5),
            suggest_on_empty: false,
        }
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }

    pub fn with_initial_value(mut self, initial_value: &str) -> Self {
        self.initial_value = initial_value.to_string();
        self
    }

    pub fn with_hint(mut self, hint: String) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn with_auto_completion(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = Some(suggestions);
        self
    }

    pub fn with_max_suggestions(mut self, max_suggestions: usize) -> Self {
        self.max_suggestions = Some(max_suggestions);
        self
    }

    pub fn with_suggest_on_empty(mut self, suggest_on_empty: bool) -> Self {
        self.suggest_on_empty = suggest_on_empty;
        self
    }

    fn match_suggestion(&self, input: &str) -> Option<Vec<String>> {
        if let Some(suggestions) = &self.suggestions {
            let matches: Vec<String> = suggestions
                .clone()
                .into_iter()
                .filter(|s| s.to_lowercase().starts_with(&input.to_lowercase()) && s != input)
                .collect();
            return if matches.is_empty() {None} else {Some(matches)};
        };
        None
    }

    pub fn prompt(&self) -> Result<String, UIError> {
        let mut stdout = stdout();
        let mut text = self.initial_value.clone();
        let mut cursor = text.len();

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout, cursor::SetCursorStyle::BlinkingBar)
            .map_err(|e| UIError::Terminal(e.to_string()))?;

        let hint = match &self.hint {
            Some(hint) => format!(" [{}]", hint),
            None => "".to_string(),
        };

        let prompt = format!("{}{}: ", self.message, hint);

        let mut is_selecting_suggestion = false;
        let mut selected_suggestion: i64 = 0;

        let apply_suggestion = |is_selecting_suggestion: &mut bool, selected_suggestion: &i64, matches: &Option<Vec<String>>, text: &mut String, cursor: &mut usize| {
            if *is_selecting_suggestion {
                *is_selecting_suggestion = false;
                if let Some(suggestions) = matches {
                    *text = suggestions[*selected_suggestion as usize].clone();
                    *cursor = text.len();
                }
            }
        };

        loop {

            let matches = if text.is_empty() && !self.suggest_on_empty {None} else {self.match_suggestion(&text)};

            crossterm::execute!(
                stdout,
                MoveToColumn(0),
                Clear(terminal::ClearType::FromCursorDown),
                Print(&prompt),
                Print(&text),
                MoveToColumn((prompt.len() + cursor) as u16),
            )
            .map_err(|e| UIError::Terminal(e.to_string()))?;

            if let Some(suggestions) = &matches {
                let height = suggestions.len();
                if selected_suggestion >= height as i64 {
                    selected_suggestion = 0;
                } else if selected_suggestion < 0 {
                    selected_suggestion = height as i64 - 1;
                }
                let suggested = &suggestions[selected_suggestion as usize];
                let max_width = suggestions.iter().max_by_key(|s| s.len()).unwrap().len();

                let (_, cy) = cursor::position().map_err(|e| UIError::Terminal(e.to_string()))?;
                let (_, th) = terminal::size().map_err(|e| UIError::Terminal(e.to_string()))?;

                if cy as u16 >= th - 1 {
                    crossterm::queue!(
                        stdout,
                        ScrollUp(height as u16),
                        MoveUp(height as u16)
                    ).map_err(|e| UIError::Terminal(e.to_string()))?
                }

                if is_selecting_suggestion {
                    crossterm::queue!(
                        stdout,
                        MoveToColumn(prompt.len() as u16),
                        Print(suggested),
                    ).map_err(|e| UIError::Terminal(e.to_string()))?
                }

                for (i, s) in suggestions.iter().enumerate() {
                    let bg_color = if is_selecting_suggestion && i == selected_suggestion as usize {Color::Cyan} else {Color::DarkGrey};
                    let fg_color = if is_selecting_suggestion && i == selected_suggestion as usize {Color::Black} else {Color::Reset};
                    let dim = if is_selecting_suggestion && i == selected_suggestion as usize {Attribute::Bold} else {Attribute::Dim};
                    crossterm::queue!(
                        stdout,
                        MoveToNextLine(1),
                        MoveToColumn((prompt.len() - 1) as u16),
                        SetBackgroundColor(bg_color),
                        SetForegroundColor(fg_color),
                        SetAttribute(Attribute::Bold),
                        Print(format!(" {}", &s[..text.len()])),
                        SetAttribute(dim),
                        SetBackgroundColor(bg_color),
                        SetForegroundColor(fg_color),
                        Print(format!("{:<width$} ", &s[text.len()..], width = (max_width as i32 - text.len() as i32).max(0) as usize)),
                        SetAttribute(Attribute::Reset),
                    ).map_err(|e| UIError::Terminal(e.to_string()))?
                }
                crossterm::queue!(
                    stdout,
                    MoveUp(height as u16),
                    MoveToColumn((prompt.len() + (if is_selecting_suggestion {suggested.len()} else {cursor})) as u16),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                ).map_err(|e| UIError::Other(e.to_string()))?;
                stdout.flush().map_err(|e| UIError::Other(e.to_string()))?;
            };

            if let Event::Key(key_event) =
                event::read().map_err(|e| UIError::Terminal(e.to_string()))?
            {
                if key_event.kind != KeyEventKind::Press {
                    continue;
                }

                match key_event.code {
                    KeyCode::Tab => {
                        if let Some(_) = &matches {
                            if is_selecting_suggestion {
                                selected_suggestion += 1
                            } else {
                                is_selecting_suggestion = true;
                                selected_suggestion = 0
                            }
                            continue;
                        }
                    },
                    KeyCode::BackTab => {
                        if let Some(suggs) = &matches {
                            if is_selecting_suggestion {
                                selected_suggestion -= 1
                            } else {
                                is_selecting_suggestion = true;
                                selected_suggestion = (suggs.len() - 1) as i64
                            }
                            continue;
                        }
                    }
                    KeyCode::Char(c) => {
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            if c == 'h' {
                                let end = cursor;
                                let mut in_word = true;
                                for (i, c) in text.chars().rev().enumerate() {
                                    if text.len() - i < cursor {
                                        if !c.is_whitespace() {
                                            if in_word {
                                                continue;
                                            } else {
                                                cursor = text.len() - i;
                                                break;
                                            }
                                        } else {
                                            in_word = false
                                        }
                                    }
                                }
                                if in_word {
                                    cursor = 0;
                                }
                                text.replace_range(cursor..end, "");
                                continue;
                            } else if c == 'c' {
                                Text::render_finish(
                                    &mut stdout,
                                    &prompt,
                                    "<interrupted>",
                                    Color::Red,
                                )?;
                                crossterm::execute!(
                                    stdout,
                                    cursor::SetCursorStyle::DefaultUserShape
                                )
                                .map_err(|e| UIError::Terminal(e.to_string()))?;
                                crossterm::terminal::disable_raw_mode()
                                    .map_err(|e| UIError::Terminal(e.to_string()))?;
                                return Err(UIError::Canceled);
                            }
                        }
                        apply_suggestion(&mut is_selecting_suggestion, &selected_suggestion, &matches, &mut text, &mut cursor);
                        text.insert(cursor, c);
                        cursor += 1;
                    }
                    KeyCode::Backspace => {
                        apply_suggestion(&mut is_selecting_suggestion, &selected_suggestion, &matches, &mut text, &mut cursor);
                        if cursor > 0 {
                            text.remove(cursor - 1);
                            cursor = (cursor as i32 - 1).max(0) as usize;
                        }
                    }
                    KeyCode::Delete => {
                        apply_suggestion(&mut is_selecting_suggestion, &selected_suggestion, &matches, &mut text, &mut cursor);
                        if cursor < text.len() {
                            if key_event.modifiers == KeyModifiers::CONTROL {
                                let start = cursor;
                                let mut in_word = true;
                                for (i, c) in text.chars().enumerate() {
                                    if i > cursor {
                                        if !c.is_whitespace() {
                                            if in_word {
                                                continue;
                                            } else {
                                                cursor = i;
                                                break;
                                            }
                                        } else {
                                            in_word = false
                                        }
                                    }
                                }
                                if in_word {
                                    cursor = text.len();
                                }
                                text.replace_range(start..cursor, "");
                                cursor = start;
                                continue;
                            }
                            text.remove(cursor);
                        }
                    }
                    KeyCode::Enter => {
                        apply_suggestion(&mut is_selecting_suggestion, &selected_suggestion, &matches, &mut text, &mut cursor);
                        break;
                    }
                    KeyCode::Left => {
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            let mut in_word = true;
                            for (i, c) in text.chars().rev().enumerate() {
                                if text.len() - i <= cursor {
                                    if !c.is_whitespace() {
                                        if in_word {
                                            continue;
                                        } else {
                                            cursor = text.len() - i;
                                            break;
                                        }
                                    } else {
                                        in_word = false
                                    }
                                }
                            }
                            if in_word {
                                cursor = 0;
                            }
                            continue;
                        }
                        cursor = (cursor as i32 - 1).max(0) as usize;
                    }
                    KeyCode::Right => {
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            let mut in_word = true;
                            for (i, c) in text.chars().enumerate() {
                                if i >= cursor {
                                    if !c.is_whitespace() {
                                        if in_word {
                                            continue;
                                        } else {
                                            cursor = i;
                                            break;
                                        }
                                    } else {
                                        in_word = false
                                    }
                                }
                            }
                            if in_word {
                                cursor = text.len();
                            }
                            continue;
                        }
                        cursor = (cursor as i32 + 1).min(text.len() as i32) as usize;
                    }
                    KeyCode::Esc => {
                        if is_selecting_suggestion {
                            is_selecting_suggestion = false;
                            continue
                        }
                        Text::render_finish(&mut stdout, &prompt, "<canceled>", Color::Red)?;
                        crossterm::execute!(stdout, cursor::SetCursorStyle::DefaultUserShape).map_err(|e| UIError::Terminal(e.to_string()))?;
                        crossterm::terminal::disable_raw_mode()
                            .map_err(|e| UIError::Terminal(e.to_string()))?;
                        return Err(UIError::Canceled);
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }

        Text::render_finish(&mut stdout, &prompt, &text, Color::Green)?;

        crossterm::execute!(stdout, cursor::SetCursorStyle::DefaultUserShape)
            .map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::terminal::disable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;

        Ok(text)
    }

    fn render_finish(
        stdout: &mut Stdout,
        prompt: &'a str,
        text: &str,
        color: Color,
    ) -> Result<(), UIError> {
        crossterm::execute!(
            stdout,
            MoveToColumn(0),
            Clear(terminal::ClearType::FromCursorDown),
            Print(prompt),
            SetForegroundColor(color),
            Print(format!("{}\n", text)),
            MoveToColumn(0),
            ResetColor,
        )
        .map_err(|e| UIError::Terminal(e.to_string()))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_input() {
        let text = Text::new("Type something please")
            .with_initial_value("Init")
            .with_auto_completion(vec![
                "Minecraft".to_owned(),
                "Modern Warfare".to_owned(),
                "Model".to_owned(),
            ])
            .prompt();

        match text {
            Ok(str) => println!("You typed: {}", str),
            Err(err) => println!("{:?}", err),
        }
    }
}
