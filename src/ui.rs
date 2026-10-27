use std::{
    collections::HashSet,
    io::{Stdout, Write, stdout},
    ops::Range,
};

use crossterm::{
    cursor::{self, MoveDown, MoveToColumn, MoveToNextLine, MoveUp},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    style::{
        Attribute::{self},
        Attributes, Color, Print, ResetColor, SetAttribute, SetAttributes, SetBackgroundColor,
        SetForegroundColor, StyledContent, Stylize,
    },
    terminal::{self, Clear, ScrollUp},
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

    fn match_suggestion(&self, input: &str) -> Option<Vec<String>> {
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

        let prompt = format!("{}{}: ", &self.message, &hint);

        let hint = hint.cyan().italic();

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

        loop {
            let matches = if text.is_empty() && !self.suggest_on_empty {
                None
            } else {
                self.match_suggestion(&text)
            };

            crossterm::queue!(
                stdout,
                MoveToColumn(0),
                Clear(terminal::ClearType::FromCursorDown),
                Print(&self.message),
                Print(&hint),
                Print(": "),
                Print(&text),
                MoveToColumn((prompt.len() + cursor) as u16),
            )
            .map_err(|e| UIError::Terminal(e.to_string()))?;

            if let Some(suggestions) = &matches {
                let max_height = self.max_suggestions;
                let height = if max_height.is_some() {
                    max_height.unwrap().min(suggestions.len())
                } else {
                    suggestions.len()
                };
                if selected_suggestion >= (selection_scroll + height) as i64 {
                    if selected_suggestion < suggestions.len() as i64
                        && max_height
                            .is_some_and(|m| selected_suggestion >= (selection_scroll + m) as i64)
                    {
                        selection_scroll = (selected_suggestion as usize / max_height.unwrap())
                            * max_height.unwrap()
                    } else {
                        selected_suggestion = 0;
                        selection_scroll = 0;
                    }
                } else if selected_suggestion < selection_scroll as i64 {
                    if selected_suggestion >= 0
                        && max_height.is_some()
                        && selected_suggestion < selection_scroll as i64
                    {
                        selection_scroll = (selected_suggestion as usize / max_height.unwrap())
                            * max_height.unwrap()
                    } else {
                        selected_suggestion = suggestions.len() as i64 - 1;
                        if max_height.is_some() {
                            selection_scroll = (selected_suggestion as usize / max_height.unwrap())
                                * max_height.unwrap()
                        }
                    }
                }
                let suggested = &suggestions[selected_suggestion as usize];
                let max_width = suggestions.iter().max_by_key(|s| s.len()).unwrap().len();

                let (_, cy) = cursor::position().map_err(|e| UIError::Terminal(e.to_string()))?;
                let (_, th) = terminal::size().map_err(|e| UIError::Terminal(e.to_string()))?;

                let space_needed = (cy as i32 + height as i32 - th as i32 + 1).max(0) as u16;

                if space_needed > 0 {
                    crossterm::queue!(stdout, ScrollUp(space_needed), MoveUp(space_needed))
                        .map_err(|e| UIError::Terminal(e.to_string()))?
                }

                if is_selecting_suggestion {
                    crossterm::queue!(stdout, MoveToColumn(prompt.len() as u16), Print(suggested),)
                        .map_err(|e| UIError::Terminal(e.to_string()))?
                }

                for (i, s) in suggestions.iter().enumerate() {
                    if self
                        .max_suggestions
                        .is_some_and(|m| !(selection_scroll..(selection_scroll + m)).contains(&i))
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
                    let scroll_color = if max_height.is_some_and(|m| {
                        (i - selection_scroll)
                            == ((selection_scroll as f64
                                / ((suggestions.len() / m) as f64 * m as f64 - m as f64))
                                * (m - 1) as f64) as usize
                    }) {
                        Color::White
                    } else {
                        Color::DarkGrey
                    };
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
                        Print(format!(
                            "{:<width$} ",
                            &s[text.len()..],
                            width = (max_width as i32 - text.len() as i32).max(0) as usize
                        )),
                        ResetColor,
                        SetForegroundColor(scroll_color),
                        Print(format!(
                            "{}",
                            if max_height.is_some_and(|m| suggestions.len() >= m) {
                                "▌"
                            } else {
                                ""
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
                        (prompt.len()
                            + (if is_selecting_suggestion {
                                suggested.len()
                            } else {
                                cursor
                            })) as u16
                    ),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                )
                .map_err(|e| UIError::Other(e.to_string()))?;
            };

            stdout.flush().map_err(|e| UIError::Other(e.to_string()))?;

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
                                    &self.message,
                                    &hint,
                                    "<interrupted>",
                                    Color::Red,
                                    None,
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
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        text.insert(cursor, c);
                        cursor += 1;
                    }
                    KeyCode::Backspace => {
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
                        if cursor > 0 {
                            text.remove(cursor - 1);
                            cursor = (cursor as i32 - 1).max(0) as usize;
                        }
                    }
                    KeyCode::Delete => {
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
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
                        apply_suggestion(
                            &mut is_selecting_suggestion,
                            &mut selected_suggestion,
                            &matches,
                            &mut text,
                            &mut cursor,
                        );
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
                            continue;
                        }
                        Text::render_finish(
                            &mut stdout,
                            &self.message,
                            &hint,
                            "<canceled>",
                            Color::Red,
                            None,
                        )?;
                        crossterm::execute!(stdout, cursor::SetCursorStyle::DefaultUserShape)
                            .map_err(|e| UIError::Terminal(e.to_string()))?;
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

        Text::render_finish(
            &mut stdout,
            &self.message,
            &hint,
            &text,
            Color::Green,
            Some(
                Attributes::default()
                    .with(Attribute::Bold)
                    .with(Attribute::Underlined),
            ),
        )?;

        crossterm::execute!(stdout, cursor::SetCursorStyle::DefaultUserShape)
            .map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::terminal::disable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;

        Ok(text)
    }

    fn render_finish(
        stdout: &mut Stdout,
        message: &'a str,
        hint: &'a StyledContent<String>,
        text: &str,
        color: Color,
        attributes: Option<Attributes>,
    ) -> Result<(), UIError> {
        crossterm::execute!(
            stdout,
            MoveToColumn(0),
            Clear(terminal::ClearType::FromCursorDown),
            Print(&message),
            Print(&hint),
            Print(": "),
            SetAttributes(if attributes.is_some() {
                attributes.unwrap()
            } else {
                Attributes::default()
            }),
            SetForegroundColor(color),
            Print(format!("{}\n", text)),
            MoveToColumn(0),
            ResetColor,
        )
        .map_err(|e| UIError::Terminal(e.to_string()))
    }
}

struct SelectItem<T> {
    data: T,
    group: Option<String>,
}

pub struct Select<'a, T> {
    message: &'a str,
    items: Vec<SelectItem<&'a mut T>>,
    formatter: fn(&T) -> String,
    group_items: bool,
    hint: Option<String>,
    page_size: Option<usize>,
    initial_selection: usize,
    filter_input: bool,
    initial_filter: String,
}

impl<'a, T> Select<'a, T> {
    pub fn new(message: &'a str, formatter: fn(&T) -> String) -> Self {
        Self {
            message,
            items: Vec::new(),
            formatter,
            group_items: false,
            hint: None,
            page_size: None,
            initial_selection: 0,
            filter_input: true,
            initial_filter: String::new(),
        }
    }

    pub fn with_group_items(mut self, group_items: bool) -> Self {
        self.group_items = group_items;
        self
    }

    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = Some(page_size);
        self
    }

    pub fn with_initial_selection(mut self, initial_selection: usize) -> Self {
        self.initial_selection = initial_selection;
        self
    }

    pub fn with_filter_input(mut self, filter_input: bool) -> Self {
        self.filter_input = filter_input;
        self
    }

    pub fn with_initial_filter(mut self, initial_filter: &str) -> Self {
        self.initial_filter = initial_filter.to_string();
        self
    }

    pub fn add_items(&mut self, items: Vec<&'a mut T>, group: Option<&str>) {
        self.items.append(
            &mut items
                .into_iter()
                .map(|item| SelectItem {
                    data: item,
                    group: group.map(|g| g.to_string()),
                })
                .collect(),
        );
    }

    pub fn add_item(&mut self, item: &'a mut T, group: Option<&str>) {
        self.items.push(SelectItem {
            data: item,
            group: group.map(|g| g.to_string()),
        });
    }

    fn count_group(&self, items: Vec<&SelectItem<&'a mut T>>) -> usize {
        if self.group_items {
            items
                .iter()
                .map(|item| &item.group)
                .collect::<HashSet<_>>()
                .iter()
                .count()
        } else {
            0
        }
    }

    pub fn prompt(mut self) -> Result<(usize, &'a mut T), UIError> {
        let mut stdout = stdout();
        let mut filter = self.initial_filter.clone();
        let mut filter_cursor = 0;
        let mut filter_mode = false;
        let mut page_index = 0;
        let mut page_ranges: Vec<Range<usize>> = Vec::new();
        let base_group_count = self.count_group(self.items.iter().map(|i| i).collect());

        let match_filter = |item: &SelectItem<&'a mut T>, filter: &str| -> bool {
            (self.formatter)(&item.data)
                .to_lowercase()
                .contains(&filter.to_lowercase())
        };

        let mut items: Vec<usize> = self.items.iter().enumerate().map(|(i, _)| i).collect();

        let calculate_page_ranges =
            |page_size: &Option<usize>, items: &Vec<usize>, page_ranges: &mut Vec<Range<usize>>| {
                page_ranges.clear();
                if page_size.is_some() {
                    let mut group = None;
                    let mut item_count: usize = 0;
                    let mut start = 0;
                    let mut end = 0;
                    for &i in items.iter() {
                        if self.group_items && self.items[i].group != group {
                            group = self.items[i].group.clone();
                            item_count += 1;
                        }
                        if item_count >= self.page_size.unwrap() {
                            page_ranges.push(start..end);
                            item_count = 0;
                            group = None;
                            start = end
                        }
                        item_count += 1;
                        end += 1;
                        if item_count >= self.page_size.unwrap() {
                            page_ranges.push(start..end);
                            item_count = 0;
                            group = None;
                            start = end
                        }
                    }
                    page_ranges.push(start..end);
                } else {
                    page_ranges.push(0..items.len());
                }
            };

        calculate_page_ranges(&self.page_size, &items, &mut page_ranges);

        let mut max_page_index;

        let hint = match &self.hint {
            Some(hint) => format!(" [{}]", hint),
            None => "".to_string(),
        };

        let prompt = format!("{}{}: ", &self.message, &hint);

        let hint = hint.cyan().italic();

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout, cursor::SetCursorStyle::BlinkingBar)
            .map_err(|e| UIError::Terminal(e.to_string()))?;

        let mut selected_index: i64 = self.initial_selection as i64;

        loop {
            items = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if match_filter(item, &filter) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();

            calculate_page_ranges(&self.page_size, &items, &mut page_ranges);

            max_page_index = page_ranges.len() - 1;
            page_index = page_index.clamp(0, max_page_index);

            let item_slice = &items[page_ranges[page_index].clone()];
            let min_height = item_slice.len()
                + self.count_group(item_slice.iter().map(|&i| &self.items[i]).collect());
            let height = self
                .page_size
                .unwrap_or(&self.items.len() + base_group_count)
                .min(min_height);

            if items.is_empty() {
                selected_index = 0;
            } else if selected_index < 0 {
                if page_index > 0 {
                    page_index -= 1;
                } else {
                    page_index = max_page_index;
                }
                selected_index = (page_ranges[page_index].len() - 1) as i64;
                continue;
            } else if selected_index >= item_slice.len() as i64 {
                if page_index < max_page_index {
                    page_index += 1;
                } else {
                    selected_index = (item_slice.len() - 1) as i64;
                    continue
                }
                selected_index = 0;
                continue;
            }

            let (_, cy) = cursor::position().map_err(|e| UIError::Terminal(e.to_string()))?;
            let (_, th) = terminal::size().map_err(|e| UIError::Terminal(e.to_string()))?;

            let space_needed = (cy as i32
                + (self
                    .page_size
                    .unwrap_or(self.items.len() + base_group_count)
                    + 1) as i32
                - (th as i32 - 1))
                .max(0) as u16;

            if space_needed > 0 {
                crossterm::queue!(stdout, ScrollUp(space_needed), MoveUp(space_needed))
                    .map_err(|e| UIError::Terminal(e.to_string()))?
            }

            if filter_mode {
                crossterm::execute!(stdout, cursor::Show)
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            } else {
                crossterm::execute!(stdout, cursor::Hide)
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            }

            let page = if self.page_size.is_some() {
                format!("Page {}/{} | ", page_index + 1, max_page_index + 1)
            } else {
                String::new()
            };

            crossterm::queue!(
                stdout,
                MoveToColumn(0),
                Clear(terminal::ClearType::FromCursorDown),
                Print(&self.message),
                Print(&hint),
                Print(": "),
                Print(if !filter_mode && filter.is_empty() {
                    "Type to filter".to_string().dim().italic().dark_cyan()
                } else {
                    filter.clone().reset()
                }),
                MoveToNextLine(
                    (self
                        .page_size
                        .unwrap_or(self.items.len() + base_group_count)
                        + 1) as u16
                ),
                SetForegroundColor(Color::Cyan),
                if filter_mode {
                    Print(format!("| {}Mode: Typing Filter | [Esc] exit typing", page))
                } else if !filter.is_empty() {
                    Print(format!(
                        "| {}Mode: Filter        | [Esc] clear filter",
                        page
                    ))
                } else {
                    Print(format!("| {}Mode: Normal        | [Esc] cancel", page))
                },
                ResetColor,
                MoveUp(
                    (self
                        .page_size
                        .unwrap_or(self.items.len() + base_group_count)
                        + 1) as u16
                ),
            )
            .map_err(|e| UIError::Terminal(e.to_string()))?;

            let mut group = None;
            let mut result = 0;

            for (i, &item_i) in items[page_ranges[page_index].clone()].iter().enumerate() {
                let item = &self.items[item_i];
                result += 1;
                if self.group_items {
                    match &item.group {
                        Some(item_group) => {
                            if Some(item_group) != group {
                                group = Some(item_group);
                                crossterm::queue!(
                                    stdout,
                                    MoveToNextLine(1),
                                    Print(format!(
                                        " {}",
                                        item_group.clone().cyan().italic().bold()
                                    )),
                                )
                                .map_err(|e| UIError::Other(e.to_string()))?
                            }
                        }
                        _ => {}
                    }
                }
                let selected = i == selected_index as usize;
                let item_string = (self.formatter)(&item.data);
                let color = if selected {
                    Color::Yellow
                } else {
                    Color::Reset
                };
                let attributes = if selected {
                    Attributes::default().with(Attribute::Bold)
                } else {
                    Attributes::none()
                };
                crossterm::queue!(
                    stdout,
                    MoveToNextLine(1),
                    SetAttributes(attributes),
                    SetForegroundColor(color),
                    Print(if selected { "  > " } else { "    " }),
                    Print(format!("{}", item_string)),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                )
                .map_err(|e| UIError::Other(e.to_string()))?;
            }

            crossterm::queue!(
                stdout,
                MoveUp(height as u16),
                MoveToColumn((prompt.len() + filter_cursor) as u16)
            )
            .map_err(|e| UIError::Other(e.to_string()))?;

            if items.is_empty() {
                crossterm::queue!(stdout, MoveDown(height as u16),)
                    .map_err(|e| UIError::Other(e.to_string()))?;
            }

            stdout.flush().map_err(|e| UIError::Other(e.to_string()))?;

            if let Event::Key(key_event) =
                event::read().map_err(|e| UIError::Terminal(e.to_string()))?
            {
                if !key_event.is_press() {
                    continue;
                }
                match key_event.code {
                    KeyCode::Up | KeyCode::BackTab => selected_index -= 1,
                    KeyCode::Down | KeyCode::Tab => {
                        selected_index += 1;
                        if selected_index >= (item_slice.len()) as i64 && page_index == max_page_index {
                            page_index = 0;
                            selected_index = 0;
                        }
                    },
                    KeyCode::Enter => {
                        if result == 0 {
                            continue;
                        }
                        break;
                    }
                    KeyCode::Esc => {
                        if filter_mode {
                            filter_mode = false;
                            continue;
                        } else if !filter.is_empty() {
                            filter.clear();
                            filter_cursor = 0;
                            continue;
                        }
                        Select::<T>::render_finish(
                            &mut stdout,
                            &self.message,
                            &hint,
                            "<canceled>",
                            Color::Red,
                            None,
                        )?;
                        crossterm::execute!(
                            stdout,
                            cursor::SetCursorStyle::DefaultUserShape,
                            cursor::Show
                        )
                        .map_err(|e| UIError::Terminal(e.to_string()))?;
                        crossterm::terminal::disable_raw_mode()
                            .map_err(|e| UIError::Terminal(e.to_string()))?;
                        return Err(UIError::Canceled);
                    }
                    KeyCode::Char(c) => {
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            if c == 'h' && self.filter_input {
                                let end = filter_cursor;
                                let mut in_word = true;
                                for (i, c) in filter.chars().rev().enumerate() {
                                    if filter.len() - i < filter_cursor {
                                        if !c.is_whitespace() {
                                            if in_word {
                                                continue;
                                            } else {
                                                filter_cursor = filter.len() - i;
                                                break;
                                            }
                                        } else {
                                            in_word = false
                                        }
                                    }
                                }
                                if in_word {
                                    filter_cursor = 0;
                                }
                                filter.replace_range(filter_cursor..end, "");
                                continue;
                            } else if c == 'c' {
                                Select::<T>::render_finish(
                                    &mut stdout,
                                    &self.message,
                                    &hint,
                                    "<interupted>",
                                    Color::Red,
                                    None,
                                )?;
                                crossterm::execute!(
                                    stdout,
                                    cursor::SetCursorStyle::DefaultUserShape,
                                    cursor::Show
                                )
                                .map_err(|e| UIError::Terminal(e.to_string()))?;
                                crossterm::terminal::disable_raw_mode()
                                    .map_err(|e| UIError::Terminal(e.to_string()))?;
                                return Err(UIError::Canceled);
                            }
                        }
                        if !self.filter_input {
                            continue;
                        }
                        filter_mode = true;
                        filter.insert(filter_cursor, c);
                        filter_cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if !self.filter_input {
                            continue;
                        }
                        filter_mode = true;
                        if filter_cursor > 0 {
                            filter.remove(filter_cursor - 1);
                            filter_cursor = (filter_cursor as i32 - 1).max(0) as usize;
                        }
                    }
                    KeyCode::Delete => {
                        if !self.filter_input {
                            continue;
                        }
                        filter_mode = true;
                        if filter_cursor < filter.len() {
                            if key_event.modifiers == KeyModifiers::CONTROL {
                                let start = filter_cursor;
                                let mut in_word = true;
                                for (i, c) in filter.chars().enumerate() {
                                    if i > filter_cursor {
                                        if !c.is_whitespace() {
                                            if in_word {
                                                continue;
                                            } else {
                                                filter_cursor = i;
                                                break;
                                            }
                                        } else {
                                            in_word = false
                                        }
                                    }
                                }
                                if in_word {
                                    filter_cursor = filter.len();
                                }
                                filter.replace_range(start..filter_cursor, "");
                                filter_cursor = start;
                                continue;
                            }
                            filter.remove(filter_cursor);
                        }
                    }
                    KeyCode::Left => {
                        if !filter_mode || filter.is_empty() {
                            if page_index as i64 - 1 < 0 {
                                page_index = max_page_index;
                                continue;
                            }
                            page_index = (page_index as i64 - 1) as usize;
                            continue;
                        }
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            let mut in_word = true;
                            for (i, c) in filter.chars().rev().enumerate() {
                                if filter.len() - i <= filter_cursor {
                                    if !c.is_whitespace() {
                                        if in_word {
                                            continue;
                                        } else {
                                            filter_cursor = filter.len() - i;
                                            break;
                                        }
                                    } else {
                                        in_word = false
                                    }
                                }
                            }
                            if in_word {
                                filter_cursor = 0;
                            }
                            continue;
                        }
                        filter_cursor = (filter_cursor as i32 - 1).max(0) as usize;
                    }
                    KeyCode::Right => {
                        if !filter_mode || filter.is_empty() {
                            page_index = (page_index + 1) as usize;
                            if page_index > max_page_index {
                                page_index = 0;
                            }
                            continue;
                        }
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            let mut in_word = true;
                            for (i, c) in filter.chars().enumerate() {
                                if i >= filter_cursor {
                                    if !c.is_whitespace() {
                                        if in_word {
                                            continue;
                                        } else {
                                            filter_cursor = i;
                                            break;
                                        }
                                    } else {
                                        in_word = false
                                    }
                                }
                            }
                            if in_word {
                                filter_cursor = filter.len();
                            }
                            continue;
                        }
                        filter_cursor =
                            (filter_cursor as i32 + 1).min(filter.len() as i32) as usize;
                    }
                    _ => {}
                }
            }
        }

        let selected_item_index = items[page_ranges[page_index].clone()][selected_index as usize];
        let selected_item = self.items.swap_remove(selected_item_index).data;

        Select::<T>::render_finish(
            &mut stdout,
            &self.message,
            &hint,
            &(self.formatter)(&selected_item),
            Color::Green,
            Some(
                Attributes::default()
                    .with(Attribute::Bold)
                    .with(Attribute::Underlined),
            ),
        )?;

        crossterm::execute!(
            stdout,
            cursor::SetCursorStyle::DefaultUserShape,
            cursor::Show
        )
        .map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::terminal::disable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;

        Ok((selected_item_index, selected_item))
    }

    fn render_finish(
        stdout: &mut Stdout,
        message: &'a str,
        hint: &'a StyledContent<String>,
        text: &str,
        color: Color,
        attributes: Option<Attributes>,
    ) -> Result<(), UIError> {
        crossterm::execute!(
            stdout,
            MoveToColumn(0),
            Clear(terminal::ClearType::FromCursorDown),
            Print(&message),
            Print(&hint),
            Print(": "),
            SetAttributes(if attributes.is_some() {
                attributes.unwrap()
            } else {
                Attributes::default()
            }),
            SetForegroundColor(color),
            Print(format!("{}\n", text)),
            MoveToColumn(0),
            ResetColor,
        )
        .map_err(|e| UIError::Terminal(e.to_string()))
    }
}

pub struct MultiSelect<'a, T> {
    message: &'a str,
    items: Vec<SelectItem<&'a mut T>>,
    formatter: fn(&T) -> String,
    selected_color: Color,
    group_items: bool,
    hint: Option<String>,
    page_size: Option<usize>,
    initial_selections: Vec<usize>,
    filter_input: bool,
    initial_filter: String,
}

pub struct MultiSelectResult<'a, T> {
    pub selected: Vec<(usize, &'a mut T)>,
    pub unselected: Vec<(usize, &'a mut T)>,
}

impl<'a, T> MultiSelect<'a, T> {
    pub fn new(message: &'a str, formatter: fn(&T) -> String) -> Self {
        Self {
            message,
            items: Vec::new(),
            formatter,
            selected_color: Color::Green,
            hint: None,
            group_items: false,
            page_size: None,
            initial_selections: Vec::new(),
            filter_input: true,
            initial_filter: String::new(),
        }
    }

    pub fn with_group_items(mut self, group_items: bool) -> Self {
        self.group_items = group_items;
        self
    }

    pub fn with_selected_color(mut self, selected_color: Color) -> Self {
        self.selected_color = selected_color;
        self
    }

    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = Some(page_size);
        self
    }

    pub fn with_initial_selection(mut self, initial_selection: Vec<usize>) -> Self {
        self.initial_selections = initial_selection;
        self
    }

    pub fn with_filter_input(mut self, filter_input: bool) -> Self {
        self.filter_input = filter_input;
        self
    }

    pub fn with_initial_filter(mut self, initial_filter: &str) -> Self {
        self.initial_filter = initial_filter.to_string();
        self
    }

    pub fn add_items(&mut self, items: Vec<&'a mut T>, group: Option<&str>) {
        self.items.append(
            &mut items
                .into_iter()
                .map(|item| SelectItem {
                    data: item,
                    group: group.map(|g| g.to_string()),
                })
                .collect(),
        );
    }

    pub fn add_item(&mut self, item: &'a mut T, group: Option<&str>) {
        self.items.push(SelectItem {
            data: item,
            group: group.map(|g| g.to_string()),
        });
    }

    fn count_group(&self, items: Vec<&SelectItem<&'a mut T>>) -> usize {
        if self.group_items {
            items
                .iter()
                .map(|item| &item.group)
                .collect::<HashSet<_>>()
                .iter()
                .count()
        } else {
            0
        }
    }

    pub fn prompt(self) -> Result<MultiSelectResult<'a, T>, UIError> {
        let mut stdout = stdout();
        let mut filter = self.initial_filter.clone();
        let mut filter_cursor = 0;
        let mut filter_mode = false;
        let mut page_index = 0;
        let mut page_ranges: Vec<Range<usize>> = Vec::new();
        let base_group_count = self.count_group(self.items.iter().map(|i| i).collect());

        let match_filter = |item: &SelectItem<&'a mut T>, filter: &str| -> bool {
            (self.formatter)(&item.data)
                .to_lowercase()
                .contains(&filter.to_lowercase())
        };

        let mut items: Vec<usize> = self.items.iter().enumerate().map(|(i, _)| i).collect();

        let calculate_page_ranges =
            |page_size: &Option<usize>, items: &Vec<usize>, page_ranges: &mut Vec<Range<usize>>| {
                page_ranges.clear();
                if page_size.is_some() {
                    let mut group = None;
                    let mut item_count: usize = 0;
                    let mut start = 0;
                    let mut end = 0;
                    for &i in items.iter() {
                        if self.group_items && self.items[i].group != group {
                            group = self.items[i].group.clone();
                            item_count += 1;
                        }
                        if item_count >= self.page_size.unwrap() {
                            page_ranges.push(start..end);
                            item_count = 0;
                            group = None;
                            start = end
                        }
                        item_count += 1;
                        end += 1;
                        if item_count >= self.page_size.unwrap() {
                            page_ranges.push(start..end);
                            item_count = 0;
                            group = None;
                            start = end
                        }
                    }
                    page_ranges.push(start..end);
                } else {
                    page_ranges.push(0..items.len());
                }
            };

        calculate_page_ranges(&self.page_size, &items, &mut page_ranges);

        let mut max_page_index;

        let hint = match &self.hint {
            Some(hint) => format!(" [{}]", hint),
            None => "".to_string(),
        };

        let prompt = format!("{}{}: ", &self.message, &hint);

        let hint = hint.cyan().italic();

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout, cursor::SetCursorStyle::BlinkingBar)
            .map_err(|e| UIError::Terminal(e.to_string()))?;

        let mut selected_index: i64 = 0;
        let mut selected_indices: HashSet<usize> =
            self.initial_selections.iter().cloned().collect();

        loop {
            items = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if match_filter(item, &filter) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();

            calculate_page_ranges(&self.page_size, &items, &mut page_ranges);

            max_page_index = page_ranges.len() - 1;
            page_index = page_index.clamp(0, max_page_index);

            let item_slice = &items[page_ranges[page_index].clone()];
            let min_height = item_slice.len() + self.count_group(item_slice.iter().map(|&i| &self.items[i]).collect());
            let height = self
                .page_size
                .unwrap_or(&self.items.len() + base_group_count)
                .min(min_height);

            if items.is_empty() {
                selected_index = 0;
            } else if selected_index < 0 {
                if page_index > 0 {
                    page_index -= 1;
                } else {
                    page_index = max_page_index;
                }
                selected_index = (page_ranges[page_index].len() - 1) as i64;
                continue;
            } else if selected_index >= item_slice.len() as i64 {
                if page_index < max_page_index {
                    page_index += 1;
                } else {
                    selected_index = (item_slice.len() - 1) as i64;
                    continue;
                }
                selected_index = 0;
                continue;
            }

            let (_, cy) = cursor::position().map_err(|e| UIError::Terminal(e.to_string()))?;
            let (_, th) = terminal::size().map_err(|e| UIError::Terminal(e.to_string()))?;

            let space_needed = (cy as i32
                + (self
                    .page_size
                    .unwrap_or(self.items.len() + base_group_count)
                    + 1) as i32
                - (th as i32 - 1))
                .max(0) as u16;

            if space_needed > 0 {
                crossterm::queue!(stdout, ScrollUp(space_needed), MoveUp(space_needed))
                    .map_err(|e| UIError::Terminal(e.to_string()))?
            }

            if filter_mode {
                crossterm::execute!(stdout, cursor::Show)
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            } else {
                crossterm::execute!(stdout, cursor::Hide)
                    .map_err(|e| UIError::Terminal(e.to_string()))?;
            }

            let page = if self.page_size.is_some() {
                format!("Page {}/{} | ", page_index + 1, max_page_index + 1)
            } else {
                String::new()
            };

            crossterm::queue!(
                stdout,
                MoveToColumn(0),
                Clear(terminal::ClearType::FromCursorDown),
                Print(&self.message),
                Print(&hint),
                Print(": "),
                Print(if !filter_mode && filter.is_empty() {
                    "Type to filter".to_string().dim().italic().dark_cyan()
                } else {
                    filter.clone().reset()
                }),
                MoveToNextLine(
                    (self
                        .page_size
                        .unwrap_or(self.items.len() + base_group_count)
                        + 1) as u16
                ),
                SetForegroundColor(Color::Cyan),
                if filter_mode {
                    Print(format!(
                        "| {}Mode: Typing Filter | [Space] Toggle | [Esc] exit typing",
                        page
                    ))
                } else if !filter.is_empty() {
                    Print(format!(
                        "| {}Mode: Filter        | [Space] Toggle | [Esc] clear filter",
                        page
                    ))
                } else {
                    Print(format!(
                        "| {}Mode: Normal        | [Space] Toggle | [Esc] cancel",
                        page
                    ))
                },
                ResetColor,
                MoveUp(
                    (self
                        .page_size
                        .unwrap_or(self.items.len() + base_group_count)
                        + 1) as u16
                ),
            )
            .map_err(|e| UIError::Terminal(e.to_string()))?;

            let mut group = None;
            let mut result = 0;

            for (i, &item_i) in items[page_ranges[page_index].clone()].iter().enumerate() {
                let item = &self.items[item_i];
                result += 1;
                if self.group_items {
                    match &item.group {
                        Some(item_group) => {
                            if Some(item_group) != group {
                                group = Some(item_group);
                                crossterm::queue!(
                                    stdout,
                                    MoveToNextLine(1),
                                    Print(format!(
                                        " {}",
                                        item_group.clone().cyan().italic().bold()
                                    )),
                                )
                                .map_err(|e| UIError::Other(e.to_string()))?
                            }
                        }
                        _ => {}
                    }
                }
                let selected = i == selected_index as usize;
                let item_string = (self.formatter)(&item.data);
                let toggled = selected_indices.contains(&item_i);
                let toggled_str = if toggled { "✔" } else { " " };
                let color = if selected {
                    Color::Yellow
                } else if toggled {
                    self.selected_color
                } else {
                    Color::Reset
                };
                let attributes = if selected {
                    Attributes::default().with(Attribute::Bold)
                } else {
                    Attributes::none()
                };
                crossterm::queue!(
                    stdout,
                    MoveToNextLine(1),
                    SetAttributes(attributes),
                    SetForegroundColor(color),
                    Print(if selected {
                        format!("  > [{}] ", toggled_str)
                    } else {
                        format!("    [{}] ", toggled_str)
                    }),
                    Print(format!("{}", item_string)),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                )
                .map_err(|e| UIError::Other(e.to_string()))?;
            }

            crossterm::queue!(
                stdout,
                MoveUp(height as u16),
                MoveToColumn((prompt.len() + filter_cursor) as u16)
            )
            .map_err(|e| UIError::Other(e.to_string()))?;

            if items.is_empty() {
                crossterm::queue!(stdout, MoveDown(height as u16),)
                    .map_err(|e| UIError::Other(e.to_string()))?;
            }

            stdout.flush().map_err(|e| UIError::Other(e.to_string()))?;

            if let Event::Key(key_event) =
                event::read().map_err(|e| UIError::Terminal(e.to_string()))?
            {
                if !key_event.is_press() {
                    continue;
                }
                match key_event.code {
                    KeyCode::Up | KeyCode::BackTab => selected_index -= 1,
                    KeyCode::Down | KeyCode::Tab => {
                        selected_index += 1;
                        if selected_index >= (item_slice.len()) as i64 && page_index == max_page_index {
                            page_index = 0;
                            selected_index = 0;
                        }
                    },
                    KeyCode::Enter => {
                        if result == 0 {
                            continue;
                        }
                        break;
                    }
                    KeyCode::Esc => {
                        if filter_mode {
                            filter_mode = false;
                            continue;
                        } else if !filter.is_empty() {
                            filter.clear();
                            filter_cursor = 0;
                            continue;
                        }
                        Select::<T>::render_finish(
                            &mut stdout,
                            &self.message,
                            &hint,
                            "<canceled>",
                            Color::Red,
                            None,
                        )?;
                        crossterm::execute!(
                            stdout,
                            cursor::SetCursorStyle::DefaultUserShape,
                            cursor::Show
                        )
                        .map_err(|e| UIError::Terminal(e.to_string()))?;
                        crossterm::terminal::disable_raw_mode()
                            .map_err(|e| UIError::Terminal(e.to_string()))?;
                        return Err(UIError::Canceled);
                    }
                    KeyCode::Char(c) => {
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            if c == 'h' && self.filter_input {
                                let end = filter_cursor;
                                let mut in_word = true;
                                for (i, c) in filter.chars().rev().enumerate() {
                                    if filter.len() - i < filter_cursor {
                                        if !c.is_whitespace() {
                                            if in_word {
                                                continue;
                                            } else {
                                                filter_cursor = filter.len() - i;
                                                break;
                                            }
                                        } else {
                                            in_word = false
                                        }
                                    }
                                }
                                if in_word {
                                    filter_cursor = 0;
                                }
                                filter.replace_range(filter_cursor..end, "");
                                continue;
                            } else if c == 'c' {
                                Select::<T>::render_finish(
                                    &mut stdout,
                                    &self.message,
                                    &hint,
                                    "<interupted>",
                                    Color::Red,
                                    None,
                                )?;
                                crossterm::execute!(
                                    stdout,
                                    cursor::SetCursorStyle::DefaultUserShape,
                                    cursor::Show
                                )
                                .map_err(|e| UIError::Terminal(e.to_string()))?;
                                crossterm::terminal::disable_raw_mode()
                                    .map_err(|e| UIError::Terminal(e.to_string()))?;
                                return Err(UIError::Canceled);
                            }
                        }
                        if !self.filter_input {
                            continue;
                        }
                        if c == ' ' && !filter_mode {
                            let to_toggle =
                                items[page_ranges[page_index].clone()][selected_index as usize];
                            if selected_indices.contains(&to_toggle) {
                                selected_indices.remove(&to_toggle);
                            } else {
                                selected_indices.insert(to_toggle);
                            }
                            continue;
                        }
                        filter_mode = true;
                        filter.insert(filter_cursor, c);
                        filter_cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if !self.filter_input {
                            continue;
                        }
                        filter_mode = true;
                        if filter_cursor > 0 {
                            filter.remove(filter_cursor - 1);
                            filter_cursor = (filter_cursor as i32 - 1).max(0) as usize;
                        }
                    }
                    KeyCode::Delete => {
                        if !self.filter_input {
                            continue;
                        }
                        filter_mode = true;
                        if filter_cursor < filter.len() {
                            if key_event.modifiers == KeyModifiers::CONTROL {
                                let start = filter_cursor;
                                let mut in_word = true;
                                for (i, c) in filter.chars().enumerate() {
                                    if i > filter_cursor {
                                        if !c.is_whitespace() {
                                            if in_word {
                                                continue;
                                            } else {
                                                filter_cursor = i;
                                                break;
                                            }
                                        } else {
                                            in_word = false
                                        }
                                    }
                                }
                                if in_word {
                                    filter_cursor = filter.len();
                                }
                                filter.replace_range(start..filter_cursor, "");
                                filter_cursor = start;
                                continue;
                            }
                            filter.remove(filter_cursor);
                        }
                    }
                    KeyCode::Left => {
                        if !filter_mode || filter.is_empty() {
                            if page_index as i64 - 1 < 0 {
                                page_index = max_page_index;
                                continue;
                            }
                            page_index = (page_index as i64 - 1) as usize;
                            continue;
                        }
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            let mut in_word = true;
                            for (i, c) in filter.chars().rev().enumerate() {
                                if filter.len() - i <= filter_cursor {
                                    if !c.is_whitespace() {
                                        if in_word {
                                            continue;
                                        } else {
                                            filter_cursor = filter.len() - i;
                                            break;
                                        }
                                    } else {
                                        in_word = false
                                    }
                                }
                            }
                            if in_word {
                                filter_cursor = 0;
                            }
                            continue;
                        }
                        filter_cursor = (filter_cursor as i32 - 1).max(0) as usize;
                    }
                    KeyCode::Right => {
                        if !filter_mode || filter.is_empty() {
                            page_index = (page_index + 1) as usize;
                            if page_index > max_page_index {
                                page_index = 0;
                            }
                            continue;
                        }
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            let mut in_word = true;
                            for (i, c) in filter.chars().enumerate() {
                                if i >= filter_cursor {
                                    if !c.is_whitespace() {
                                        if in_word {
                                            continue;
                                        } else {
                                            filter_cursor = i;
                                            break;
                                        }
                                    } else {
                                        in_word = false
                                    }
                                }
                            }
                            if in_word {
                                filter_cursor = filter.len();
                            }
                            continue;
                        }
                        filter_cursor =
                            (filter_cursor as i32 + 1).min(filter.len() as i32) as usize;
                    }
                    _ => {}
                }
            }
        }

        let list_size = self.items.len();
        let (selected_items, unselected_items): (Vec<(usize, &'a mut T)>, Vec<(usize, &'a mut T)>) =
            self.items
                .into_iter()
                .enumerate()
                .map(|(i, item)| (i, item.data))
                .partition(|(i, _)| selected_indices.contains(i));

        Select::<T>::render_finish(
            &mut stdout,
            &self.message,
            &hint,
            &format!("Selected {} out of {}", selected_items.len(), list_size),
            Color::Green,
            Some(
                Attributes::default()
                    .with(Attribute::Bold)
                    .with(Attribute::Underlined),
            ),
        )?;

        crossterm::execute!(
            stdout,
            cursor::SetCursorStyle::DefaultUserShape,
            cursor::Show
        )
        .map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::terminal::disable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;

        let result = MultiSelectResult {
            selected: selected_items,
            unselected: unselected_items,
        };

        Ok(result)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    fn get_test_vec() -> Vec<String> {
        vec![
            "Genshin Impact".to_owned(),
            "Zenless Zone Zero".to_owned(),
            "Valorant".to_owned(),
            "Counter Strike".to_owned(),
            "League of Legends".to_owned(),
            "Terraria".to_owned(),
            "Minecraft".to_owned(),
            "Call of Duty Modern Warfare".to_owned(),
            "Honkai Impact".to_owned(),
            "Honkai Star Rail".to_owned(),
            "Minecraft Dungeon".to_owned(),
            "NeiR Automata".to_owned(),
            "Battlefield".to_owned(),
            "Arknight Endfield".to_owned(),
            "Ghost of Tsushima".to_owned(),
        ]
    }

    #[test]
    fn test() {
        test_confirm();
        test_text();
        test_select();
        test_multi_select();
    }

    fn test_confirm() {
        let confirm = Confirm::new("Are you gay?").with_default(true).prompt();

        match confirm {
            Ok(yes) => println!(
                "{}",
                if yes {
                    "You are gay!"
                } else {
                    "You are straight!"
                }
            ),
            Err(err) => println!("{:?}", err),
        }
    }

    fn test_text() {
        let text = Text::new("Type something please")
            .with_initial_value("Init")
            .with_auto_completion(get_test_vec())
            .with_suggest_on_empty(true)
            // .with_max_suggestions(None)
            .with_hint("Games please")
            .prompt();

        match text {
            Ok(str) => println!("You typed: {}", str),
            Err(err) => println!("{:?}", err),
        }
    }

    fn test_select() {
        let mut select = Select::<String>::new("Select something please", |i| i.clone())
            .with_hint("Game please")
            .with_page_size(6)
            .with_group_items(true);
        let mut first = get_test_vec();
        let mut second = first.split_off(7);

        // select.add_items(first.iter_mut().collect(), None);
        select.add_items(first.iter_mut().collect(), Some("Separator desu:"));
        select.add_items(second.iter_mut().collect(), Some("Separator 2 desu:"));

        let selection = select.prompt();

        match selection {
            Ok((_, str)) => println!("You selected: {}", str),
            Err(err) => println!("{:?}", err),
        }
    }

    fn test_multi_select() {
        let mut select = MultiSelect::<String>::new("Select a few things please", |i| i.clone())
            .with_hint("Game please")
            .with_page_size(6)
            .with_group_items(true);
        let mut first = get_test_vec();
        let mut second = first.split_off(7);

        select.add_items(first.iter_mut().collect(), Some("Separator desu:"));
        select.add_items(second.iter_mut().collect(), Some("Separator 2 desu:"));

        let selections = select.with_initial_selection(vec![3, 4, 5, 6]).prompt();

        match selections {
            Ok(strs) => {
                println!("You have selected:");
                for (_, str) in strs.selected {
                    println!("{}", str)
                }
            }
            Err(err) => println!("{:?}", err),
        }
    }
}
