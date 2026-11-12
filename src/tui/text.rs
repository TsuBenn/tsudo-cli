use std::io::{Write, stdout};

use crate::tui::util::*;
use crossterm::{
    cursor::{self, MoveDown, MoveToColumn, MoveToNextLine, MoveToRow, MoveUp}, event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers}, style::{
        Attribute::{self},
        Attributes, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
        Stylize,
    }, terminal::{self, ScrollUp},
};
use textwrap::Options;

pub struct Text<'a> {
    message: &'a str,
    default: Option<String>,
    validation: Option<fn(&str) -> (bool, Option<String>)>,
    hint: Option<String>,
    initial_value: String,
    max_width: u16,
    suggestions: Option<Vec<String>>,
    max_suggestions: Option<usize>,
    suggest_on_empty: bool,
}

impl<'a> Text<'a> {
    pub fn new(message: &'a str) -> Self {
        Self {
            message,
            default: None,
            validation: None,
            hint: None,
            max_width: 80,
            initial_value: String::new(),
            suggestions: None,
            max_suggestions: Some(5),
            suggest_on_empty: false,
        }
    }

    pub fn with_max_width(mut self, max_width: u16) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn with_validation(mut self, validation: fn(&str) -> (bool, Option<String>)) -> Self {
        self.validation = Some(validation);
        self
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }

    pub fn with_initial_value(mut self, initial_value: &str) -> Self {
        self.initial_value = initial_value.to_string();
        self
    }

    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    pub fn with_auto_completion(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = Some(suggestions);
        self
    }

    pub fn with_max_suggestions(mut self, max_suggestions: Option<usize>) -> Self {
        self.max_suggestions = max_suggestions;
        self
    }

    pub fn with_suggest_on_empty(mut self, suggest_on_empty: bool) -> Self {
        self.suggest_on_empty = suggest_on_empty;
        self
    }

    pub fn match_suggestion(&self, input: &str) -> Option<Vec<String>> {
        if let Some(suggestions) = &self.suggestions {
            let matches: Vec<String> = suggestions
                .clone()
                .into_iter()
                .filter(|s| s.to_lowercase().starts_with(&input.to_lowercase()) && s != input)
                .collect();
            return if matches.is_empty() {
                None
            } else {
                Some(matches)
            };
        };
        None
    }

    pub fn delete_word_back(cursor: &mut usize, text: &mut String) {
        let end = *cursor;
        let mut in_word = true;
        for (i, c) in text.chars().rev().enumerate() {
            if text.len() - i <= *cursor {
                if !c.is_whitespace() && c != '-' && c != '.' && c != '/' {
                    if in_word {
                        continue;
                    } else {
                        *cursor = text.len() - i;
                        break;
                    }
                } else {
                    in_word = false
                }
            }
        }
        if in_word {
            *cursor = 0;
        }
        (*text).replace_range(*cursor..end, "");
    }

    pub fn type_char(cursor: &mut usize, text: &mut String, c: char) {
        (*text).insert(*cursor, c);
        *cursor += 1;
    }

    pub fn delete_char_back(cursor: &mut usize, text: &mut String) {
        if *cursor > 0 {
            (*text).remove(*cursor - 1);
            *cursor = (*cursor as i32 - 1).max(0) as usize;
        }
    }

    pub fn delete_front(cursor: &mut usize, text: &mut String, key_event: &KeyEvent) {
        if key_event.modifiers == KeyModifiers::CONTROL {
            let start = *cursor;
            let mut in_word = true;
            for (i, c) in (*text).chars().enumerate() {
                if i > *cursor {
                    if !c.is_whitespace() {
                        if in_word {
                            continue;
                        } else {
                            *cursor = i;
                            break;
                        }
                    } else {
                        in_word = false
                    }
                }
            }
            if in_word {
                *cursor = text.len();
            }
            (*text).replace_range(start..*cursor, "");
            *cursor = start;
            return;
        }
        if *cursor < text.len() {
            (*text).remove(*cursor);
        }
    }

    pub fn move_left(cursor: &mut usize, text: &mut String, key_event: &KeyEvent) {
        if key_event.modifiers == KeyModifiers::CONTROL {
            let mut in_word = true;
            for (i, c) in (*text).chars().rev().enumerate() {
                if text.len() - i <= *cursor {
                    if !c.is_whitespace() && c != '-' && c != '.' && c != '/'  {
                        if in_word {
                            continue;
                        } else {
                            *cursor = text.len() - i;
                            break;
                        }
                    } else {
                        in_word = false
                    }
                }
            }
            if in_word {
                *cursor = 0;
            }
            return;
        }
        *cursor = (*cursor as i32 - 1).max(0) as usize;
    }

    pub fn move_right(cursor: &mut usize, text: &mut String, key_event: &KeyEvent) {
        if key_event.modifiers == KeyModifiers::CONTROL {
            let mut in_word = true;
            for (i, c) in text.chars().enumerate() {
                if i >= *cursor {
                    if !c.is_whitespace() && c != '-' && c != '.' && c != '/' {
                        if in_word {
                            continue;
                        } else {
                            *cursor = i;
                            break;
                        }
                    } else {
                        in_word = false
                    }
                }
            }
            if in_word {
                *cursor = text.len();
            }
            return;
        }
        *cursor = (*cursor as i32 + 1).min(text.len() as i32) as usize;
    }

    pub fn prompt(&self) -> Result<String, UIError> {
        let mut stdout = stdout();
        let mut text = self.initial_value.clone();
        let mut cursor = text.len();
        let mut invalid_message = String::new();

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout,
            cursor::SetCursorStyle::BlinkingBar,
        )
        .map_err(|e| UIError::Terminal(e.to_string()))?;

        let mut is_selecting_suggestion = false;
        let mut selected_suggestion: i64 = 0;
        let mut selection_scroll = 0;

        let apply_suggestion = |is_selecting_suggestion: &mut bool,
                                selected_suggestion: &mut i64,
                                matches: &Option<Vec<String>>,
                                text: &mut String,
                                cursor: &mut usize| {
            if *is_selecting_suggestion {
                *is_selecting_suggestion = false;
                if let Some(suggestions) = matches {
                    *text = suggestions[*selected_suggestion as usize].clone();
                    *cursor = text.len();
                    *selected_suggestion = 0;
                }
            }
        };

        let mut cursor_x;
        let mut cursor_y;
        let mut cursor_accumulate;

        loop {
            let matches = if text.is_empty() && !self.suggest_on_empty {
                None
            } else {
                self.match_suggestion(&text)
            };

            let prompt_len = render_prompt(
                &mut stdout,
                &self.message,
                &self.hint,
                "",
                Color::Reset,
                None,
                false,
            )?;

            let (_, cy) = cursor::position().map_err(|e| UIError::Terminal(e.to_string()))?;
            let (tw, th) = terminal::size().map_err(|e| UIError::Terminal(e.to_string()))?;

            let wrapped_text = if text.len() <= self.max_width.min(tw - 1) as usize {
                text.clone()
            } else {
                textwrap::fill(
                    &text,
                    Options::new(self.max_width.min(tw - 1) as usize).preserve_trailing_space(true),
                )
            };
            let text_lines = wrapped_text.split("\n").count();
            cursor_x = 0;
            cursor_y = 0;
            cursor_accumulate = 0;

            if !wrapped_text.is_empty() {
                crossterm::queue!(stdout, MoveUp(1))
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            }

            for line in wrapped_text.split("\n") {
                if cursor_accumulate < cursor {
                    cursor_y += 1;
                    cursor_x = line.len().min(cursor - cursor_accumulate);
                    cursor_accumulate += cursor_x;
                }
                crossterm::queue!(
                    stdout,
                    MoveDown(1),
                    MoveToColumn(prompt_len as u16),
                    Print(line),
                )
                .map_err(|e| UIError::Terminal(e.to_string()))?;
            }
            cursor_y = cursor_y.max(1);

            let space_needed = (cy as i32 + text_lines as i32 - th as i32).max(0) as u16;

            if space_needed > 0 {
                crossterm::queue!(stdout, ScrollUp(space_needed), MoveUp(space_needed))
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            }

            if text_lines > 1 {
                crossterm::queue!(stdout, MoveUp((text_lines as i64 - 1).max(0) as u16))
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            }

            crossterm::queue!(
                stdout,
                MoveToColumn((prompt_len + cursor_x) as u16),
                MoveToRow(cy + (cursor_y - 1) as u16)
            )
            .map_err(|e| UIError::Terminal(e.to_string()))?;

            if let Some(suggestions) = &matches {
                let max_height = self.max_suggestions;
                let height = match max_height {
                    Some(max_height) => max_height.min(suggestions.len()),
                    None => suggestions.len(),
                };
                if selected_suggestion >= (selection_scroll + height) as i64 {
                    selected_suggestion = (selection_scroll + height - 1) as i64;
                    match max_height {
                        Some(max_height)
                            if selected_suggestion >= (selection_scroll + max_height) as i64 =>
                        {
                            if selected_suggestion < suggestions.len() as i64 {
                                selection_scroll =
                                    (selected_suggestion as usize / max_height) * max_height
                            }
                        }
                        _ => {
                            selected_suggestion = 0;
                            selection_scroll = 0;
                        }
                    }
                } else if selected_suggestion < selection_scroll as i64 {
                    match max_height {
                        Some(max_height)
                            if selected_suggestion >= 0
                                && selected_suggestion < selection_scroll as i64 =>
                        {
                            selection_scroll =
                                (selected_suggestion as usize / max_height) * max_height
                        }
                        _ => {
                            selected_suggestion = suggestions.len() as i64 - 1;
                            if let Some(max_height) = max_height {
                                selection_scroll =
                                    (selected_suggestion as usize / max_height) * max_height
                            };
                        }
                    }
                }
                let suggested = &suggestions[selected_suggestion as usize];
                let max_width = suggestions.iter().max_by_key(|s| s.len()).unwrap().len();

                let space_needed = (cy as i32 + height as i32 - th as i32 + 1).max(0) as u16;

                if space_needed > 0 {
                    crossterm::queue!(stdout, ScrollUp(space_needed), MoveUp(space_needed))
                        .map_err(|e| UIError::Terminal(e.to_string()))?;
                    continue;
                }

                if is_selecting_suggestion {
                    crossterm::queue!(stdout, MoveToColumn(prompt_len as u16), Print(suggested),)
                        .map_err(|e| UIError::Terminal(e.to_string()))?
                }

                for (i, s) in suggestions.iter().enumerate() {
                    if let Some(max_suggestions) = self.max_suggestions
                        && !(selection_scroll..(selection_scroll + max_suggestions)).contains(&i)
                    {
                        continue;
                    }
                    let bg_color = if is_selecting_suggestion && i == selected_suggestion as usize {
                        Color::Cyan
                    } else {
                        Color::DarkGrey
                    };
                    let fg_color = if is_selecting_suggestion && i == selected_suggestion as usize {
                        Color::Black
                    } else {
                        Color::Reset
                    };
                    let dim = if is_selecting_suggestion && i == selected_suggestion as usize {
                        Attribute::Bold
                    } else {
                        Attribute::Dim
                    };
                    let scroll_color = match max_height {
                        Some(max_height)
                            if (i - selection_scroll)
                                == ((selection_scroll as f64
                                    / ((suggestions.len() / max_height) as f64
                                        * max_height as f64
                                        - max_height as f64))
                                    * (max_height - 1) as f64)
                                    as usize =>
                        {
                            Color::White
                        }
                        _ => Color::DarkGrey,
                    };
                    crossterm::queue!(
                        stdout,
                        MoveToNextLine(1),
                        MoveToColumn((prompt_len - 1) as u16),
                        SetBackgroundColor(bg_color),
                        SetForegroundColor(fg_color),
                        SetAttribute(Attribute::Bold),
                        Print(format!(" {}", &s[..text.len()])),
                        SetAttribute(dim),
                        SetBackgroundColor(bg_color),
                        SetForegroundColor(fg_color),
                        Print(format!(
                            "{:<width$} ",
                            &s[text.len()..],
                            width = (max_width as i32 - text.len() as i32).max(0) as usize
                        )),
                        ResetColor,
                        SetForegroundColor(scroll_color),
                        Print(format!(
                            "{}",
                            match max_height {
                                Some(m) if suggestions.len() >= m => "▌",
                                _ => "",
                            }
                        )),
                        SetAttribute(Attribute::Reset),
                    )
                    .map_err(|e| UIError::Terminal(e.to_string()))?
                }
                crossterm::queue!(
                    stdout,
                    MoveUp(height as u16),
                    MoveToColumn(
                        (prompt_len
                            + (if is_selecting_suggestion {
                                suggested.len()
                            } else {
                                cursor_x
                            })) as u16
                    ),
                    MoveToRow(cy + (cursor_y - 1) as u16),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                )
                .map_err(|e| UIError::Other(e.to_string()))?;
            };

            if !invalid_message.is_empty() {
                crossterm::queue!(
                    stdout,
                    Print(invalid_message.clone().red().bold().dim()),
                    MoveToColumn(prompt_len as u16),
                )
                .map_err(|e| UIError::Terminal(e.to_string()))?;
                invalid_message.clear();
            }

            stdout.flush().map_err(|e| UIError::Other(e.to_string()))?;

            if cursor_y > 1 {
                crossterm::queue!(stdout, MoveUp(cursor_y - 1))
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            }

            if let Event::Key(key_event) =
                event::read().map_err(|e| UIError::Terminal(e.to_string()))?
            {
                if key_event.kind != KeyEventKind::Press {
                    continue;
                }

                match key_event.code {
                    KeyCode::Tab | KeyCode::Down => {
                        if let Some(_) = &matches {
                            if is_selecting_suggestion {
                                selected_suggestion += 1
                            } else {
                                is_selecting_suggestion = true;
                                selected_suggestion = 0
                            }
                            continue;
                        }
                    }
                    KeyCode::BackTab | KeyCode::Up => {
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
                                apply_suggestion(
                                    &mut is_selecting_suggestion,
                                    &mut selected_suggestion,
                                    &matches,
                                    &mut text,
                                    &mut cursor,
                                );
                                Text::delete_word_back(&mut cursor, &mut text);
                                continue;
                            } else if c == 'c' {
                                render_prompt(
                                    &mut stdout,
                                    &self.message,
                                    &self.hint,
                                    "<interupted>",
                                    Color::Red,
                                    None,
                                    true,
                                )?;
                                cleanup_terminal(&mut stdout, true)?;
                            } else if c == 'k' {
                                Text::type_char(&mut cursor, &mut text, '\n');
                                continue
                            }
                        }
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        Text::type_char(&mut cursor, &mut text, c);
                    }
                    KeyCode::Backspace => {
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        Text::delete_char_back(&mut cursor, &mut text);
                    }
                    KeyCode::Delete => {
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        Text::delete_front(&mut cursor, &mut text, &key_event);
                        continue;
                    }
                    KeyCode::Enter => {
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        match self.validation {
                            Some(validation) => {
                                let (valid, message) = validation(&text);
                                if !valid {
                                    text.clear();
                                    cursor = 0;
                                    invalid_message =
                                        message.unwrap_or("Invalid Input!".to_owned());
                                    continue;
                                }
                            }
                            None => {}
                        }
                        break;
                    }
                    KeyCode::Left => {
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        Text::move_left(&mut cursor, &mut text, &key_event);
                        continue;
                    }
                    KeyCode::Right => {
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        Text::move_right(&mut cursor, &mut text, &key_event);
                        continue;
                    }
                    KeyCode::Esc => {
                        if is_selecting_suggestion {
                            is_selecting_suggestion = false;
                            continue;
                        }
                        render_prompt(
                            &mut stdout,
                            &self.message,
                            &self.hint,
                            "<canceled>",
                            Color::Red,
                            None,
                            true,
                        )?;
                        cleanup_terminal(&mut stdout, true)?;
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }

        render_prompt(
            &mut stdout,
            &self.message,
            &self.hint,
            &text,
            Color::Green,
            Some(
                Attributes::default()
                    .with(Attribute::Underlined)
                    .with(Attribute::Bold),
            ),
            true,
        )?;

        cleanup_terminal(&mut stdout, false)?;

        Ok(text)
    }
}
