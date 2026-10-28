use std::{
    collections::HashSet,
    i128::MAX,
    io::{Stdout, Write, stdout},
    ops::Range,
};

use crossterm::{
    Command,
    cursor::{self, MoveDown, MoveLeft, MoveRight, MoveToColumn, MoveToNextLine, MoveUp},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{
        Attribute::{self},
        Attributes, Color, Print, ResetColor, SetAttribute, SetAttributes, SetBackgroundColor,
        SetForegroundColor, Stylize,
    },
    terminal::{self, Clear, ScrollUp},
};

struct NoOp;

impl Command for NoOp {
    fn write_ansi(&self, _: &mut impl std::fmt::Write) -> std::fmt::Result {
        Ok(())
    }
}

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

fn render_prompt(
    stdout: &mut Stdout,
    message: &str,
    hint: &Option<String>,
    text: &str,
    color: Color,
    attributes: Option<Attributes>,
    new_line: bool,
) -> Result<usize, UIError> {
    let hint = match hint.as_ref() {
        Some(hint) if !hint.is_empty() => format!("[{}]", hint).cyan().to_string(),
        _ => String::new(),
    };
    let prompt = format!("{} {} {}: ", "?".to_string().green(), &message, &hint);
    crossterm::execute!(
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

fn cleanup_terminal(stdout: &mut Stdout, report_cancel: bool) -> Result<(), UIError> {
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

        let options = Some((if self.default { "Y/n" } else { "y/N" }).to_string());

        loop {
            render_prompt(
                &mut stdout,
                self.message,
                &options,
                "",
                Color::Reset,
                None,
                false,
            )?;

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
                    KeyCode::Char('c') => {
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            render_prompt(
                                &mut stdout,
                                self.message,
                                &options,
                                "<interupted>",
                                Color::Red,
                                None,
                                true,
                            )?;
                            cleanup_terminal(&mut stdout, true)?;
                        }
                    }
                    KeyCode::Esc => {
                        render_prompt(
                            &mut stdout,
                            self.message,
                            &options,
                            "<canceled>",
                            Color::Red,
                            None,
                            true,
                        )?;
                        cleanup_terminal(&mut stdout, true)?;
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

        render_prompt(
            &mut stdout,
            self.message,
            &options,
            if selection { "Yes" } else { "No" },
            Color::Green,
            Some(
                Attributes::default()
                    .with(Attribute::Bold)
                    .with(Attribute::Underlined),
            ),
            true,
        )?;

        cleanup_terminal(&mut stdout, false)?;

        Ok(selection)
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

    fn delete_word_back(cursor: &mut usize, text: &mut String) {
        let end = *cursor;
        let mut in_word = true;
        for (i, c) in text.chars().rev().enumerate() {
            if text.len() - i < *cursor {
                if !c.is_whitespace() {
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

    fn type_char(cursor: &mut usize, text: &mut String, c: char) {
        (*text).insert(*cursor, c);
        *cursor += 1;
    }

    fn delete_char_back(cursor: &mut usize, text: &mut String) {
        if *cursor > 0 {
            (*text).remove(*cursor - 1);
            *cursor = (*cursor as i32 - 1).max(0) as usize;
        }
    }

    fn delete_front(cursor: &mut usize, text: &mut String, key_event: &KeyEvent) {
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

    fn move_left(cursor: &mut usize, text: &mut String, key_event: &KeyEvent) {
        if key_event.modifiers == KeyModifiers::CONTROL {
            let mut in_word = true;
            for (i, c) in (*text).chars().rev().enumerate() {
                if text.len() - i <= *cursor {
                    if !c.is_whitespace() {
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

    fn move_right(cursor: &mut usize, text: &mut String, key_event: &KeyEvent) {
        if key_event.modifiers == KeyModifiers::CONTROL {
            let mut in_word = true;
            for (i, c) in text.chars().enumerate() {
                if i >= *cursor {
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
            return;
        }
        *cursor = (*cursor as i32 + 1).min(text.len() as i32) as usize;
    }

    pub fn prompt(&self) -> Result<String, UIError> {
        let mut stdout = stdout();
        let mut text = self.initial_value.clone();
        let mut cursor = text.len();

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout, cursor::SetCursorStyle::BlinkingBar)
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
                &text,
                Color::Reset,
                None,
                false,
            )?;

            crossterm::queue!(stdout, MoveToColumn((prompt_len + cursor) as u16))
                .map_err(|e| UIError::Terminal(e.to_string()))?;

            if let Some(suggestions) = &matches {
                let max_height = self.max_suggestions;
                let height = match max_height {
                    Some(max_height) => max_height.min(suggestions.len()),
                    None => suggestions.len(),
                };
                if selected_suggestion >= (selection_scroll + height) as i64 {
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

                let (_, cy) = cursor::position().map_err(|e| UIError::Terminal(e.to_string()))?;
                let (_, th) = terminal::size().map_err(|e| UIError::Terminal(e.to_string()))?;

                let space_needed = (cy as i32 + height as i32 - th as i32 + 1).max(0) as u16;

                if space_needed > 0 {
                    crossterm::queue!(stdout, ScrollUp(space_needed), MoveUp(space_needed))
                        .map_err(|e| UIError::Terminal(e.to_string()))?
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
                        break;
                    }
                    KeyCode::Left => {
                        Text::move_left(&mut cursor, &mut text, &key_event);
                        continue;
                    }
                    KeyCode::Right => {
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

struct SelectItem<T> {
    data: T,
    disabled: bool,
    group: Option<String>,
}

pub struct Select<'a, T> {
    message: &'a str,
    items: Vec<SelectItem<&'a mut T>>,
    formatter: fn(&T) -> String,
    selected_color: Color,
    group_items: bool,
    disabled_items: Vec<usize>,
    hint: Option<String>,
    page_size: Option<usize>,
    initial_selections: Vec<usize>,
    filter_input: bool,
    initial_filter: String,
    multi_selection: bool,
}

pub struct SelectResult<'a, T> {
    pub selected: Vec<(usize, &'a mut T)>,
    pub unselected: Vec<(usize, &'a mut T)>,
}

impl<'a, T> SelectResult<'a, T> {
    pub fn data(&'a self) -> &'a T {
        self.selected[0].1
    }
    pub fn data_mut(&'a mut self) -> &'a mut T {
        self.selected[0].1
    }
    pub fn index(&'a self) -> usize {
        self.selected[0].0
    }
}

impl<'a, T> Select<'a, T> {
    pub fn new(message: &'a str, formatter: fn(&T) -> String) -> Self {
        Self {
            message,
            items: Vec::new(),
            formatter,
            selected_color: Color::Green,
            hint: None,
            group_items: false,
            disabled_items: Vec::new(),
            page_size: None,
            initial_selections: Vec::new(),
            filter_input: true,
            initial_filter: String::new(),
            multi_selection: false,
        }
    }

    pub fn with_multi_selection(mut self, multi_selection: bool) -> Self {
        self.multi_selection = multi_selection;
        self
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

    pub fn with_disabled_items(mut self, disabled_items: Vec<usize>) -> Self {
        self.disabled_items = disabled_items.to_vec();
        self
    }

    pub fn add_items(&mut self, items: &'a mut [T], group: Option<&str>) {
        for item in items.iter_mut() {
            let select_item = SelectItem {
                data: item,
                disabled: false,
                group: group.map(|g| g.to_string()),
            };
            self.items.push(select_item)
        }
    }

    pub fn add_item(&mut self, item: &'a mut T, group: Option<&str>) {
        self.items.push(SelectItem {
            data: item,
            disabled: false,
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

    fn match_filter(&self, item: &SelectItem<&'a mut T>, filter: &str) -> bool {
        match &item.group {
            Some(group) if filter.starts_with("%") => {
                let mut group_filter = Vec::new();
                let mut item_filter = Vec::new();
                let mut group_mode = true;
                for word in filter.split_whitespace() {
                    if word == "-" {
                        group_mode = false;
                        continue;
                    }
                    if group_mode {
                        group_filter.push(word);
                    } else {
                        item_filter.push(word);
                    }
                }
                let group_filter = group_filter.join("");
                let item_filter = item_filter.join("");
                group.to_lowercase().replace(" ", "").contains(
                    &group_filter
                        .strip_prefix("%")
                        .unwrap_or("")
                        .to_lowercase()
                        .replace(" ", ""),
                ) && ((self.formatter)(&item.data)
                    .to_lowercase()
                    .replace(" ", "")
                    .contains(&item_filter.to_lowercase().replace(" ", ""))
                    || item_filter.is_empty())
            }
            _ => (self.formatter)(&item.data)
                .to_lowercase()
                .replace(" ", "")
                .contains(&filter.to_lowercase().replace(" ", "")),
        }
    }

    fn calculate_page_ranges(
        &self,
        page_size: &Option<usize>,
        items: &Vec<usize>,
        page_ranges: &mut Vec<Range<usize>>,
    ) {
        page_ranges.clear();
        match page_size {
            Some(page_size) => {
                let mut group = None;
                let mut item_count: usize = 0;
                let mut start = 0;
                let mut end = 0;
                for &i in items.iter() {
                    if self.group_items && self.items[i].group != group {
                        group = self.items[i].group.clone();
                        item_count += 1;
                    }
                    if item_count >= *page_size {
                        page_ranges.push(start..end);
                        item_count = 0;
                        group = None;
                        start = end
                    }
                    item_count += 1;
                    end += 1;
                    if item_count >= *page_size {
                        page_ranges.push(start..end);
                        item_count = 0;
                        group = None;
                        start = end
                    }
                }
                if start != end {
                    page_ranges.push(start..end);
                }
            }
            None => {
                page_ranges.push(0..items.len());
            }
        }
    }

    pub fn prompt(self) -> Result<SelectResult<'a, T>, UIError> {
        let mut stdout = stdout();
        let mut filter = self.initial_filter.clone();
        let mut filter_cursor = 0;
        let mut filter_mode = false;
        let mut page_index = 0;
        let mut page_ranges: Vec<Range<usize>> = Vec::new();
        let base_group_count = self.count_group(self.items.iter().map(|i| i).collect());
        let mut direction = 0;

        let mut items;

        let mut max_page_index;

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout, cursor::SetCursorStyle::BlinkingBar)
            .map_err(|e| UIError::Terminal(e.to_string()))?;

        let mut selected_index: i64 = 0;
        let mut selected_indices: HashSet<usize> = if self.multi_selection {
            self.initial_selections.iter().cloned().collect()
        } else {
            HashSet::new()
        };

        let toggling_hint = if self.multi_selection {
            "| [Space] Toggle "
        } else {
            ""
        };

        loop {
            items = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if self.match_filter(item, &filter) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();

            self.calculate_page_ranges(&self.page_size, &items, &mut page_ranges);

            max_page_index = (page_ranges.len() as i64 - 1).max(0) as usize;
            page_index = page_index.clamp(0, max_page_index);

            let item_slice = &items
                .get(page_ranges.get(page_index).unwrap_or(&(0..0)).clone())
                .unwrap_or(&[]);
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
                    selected_index = 0;
                } else {
                    if direction > 0 {
                        page_index = 0;
                        selected_index = 0;
                        continue;
                    }
                    selected_index = (item_slice.len() - 1) as i64;
                }
                continue;
            }

            direction = 0;

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

            let page = match self.page_size {
                Some(_) => format!("Page {}/{} | ", page_index + 1, max_page_index + 1),
                None => String::new(),
            };

            let prompt_len = render_prompt(
                &mut stdout,
                &self.message,
                &self.hint,
                if !filter_mode && filter.is_empty() {
                    "Type to Filter"
                } else {
                    &filter
                },
                if !filter_mode && filter.is_empty() {
                    Color::DarkCyan
                } else {
                    Color::Reset
                },
                if !filter_mode && filter.is_empty() {
                    Some(
                        Attributes::default()
                            .with(Attribute::Dim)
                            .with(Attribute::Italic),
                    )
                } else {
                    None
                },
                false,
            )?;

            crossterm::queue!(
                stdout,
                MoveToNextLine(
                    (self
                        .page_size
                        .unwrap_or(self.items.len() + base_group_count)
                        + 1) as u16
                ),
                SetForegroundColor(Color::Cyan),
                if filter_mode {
                    Print(format!(
                        "| {}Mode: Typing Filter {}| [Esc] exit typing",
                        page, toggling_hint
                    ))
                } else if !filter.is_empty() {
                    Print(format!(
                        "| {}Mode: Filter        {}| [Esc] clear filter",
                        page, toggling_hint
                    ))
                } else {
                    Print(format!(
                        "| {}Mode: Normal        {}| [Esc] cancel",
                        page, toggling_hint
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

            for (i, &item_i) in items
                .get(page_ranges.get(page_index).unwrap_or(&(0..0)).clone())
                .unwrap_or(&[])
                .iter()
                .enumerate()
            {
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
                let disabled = self.disabled_items.contains(&item_i);
                let item_string_formatted = format!(
                    "{}{}",
                    if self.multi_selection {
                        format!("[{}] ", toggled_str)
                    } else {
                        String::new()
                    },
                    item_string
                );
                let item_string_formatted = if disabled {
                    item_string_formatted.reset().dim().to_string()
                } else {
                    item_string_formatted
                };
                crossterm::queue!(
                    stdout,
                    MoveToNextLine(1),
                    SetAttributes(attributes),
                    SetForegroundColor(color),
                    Print(if selected {
                        format!("  > ")
                    } else {
                        format!("    ")
                    }),
                    Print(format!("{}", item_string_formatted)),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                )
                .map_err(|e| UIError::Other(e.to_string()))?;
                if filter_mode && !filter.is_empty() {
                    let mut filter = filter.clone();
                    if filter.starts_with("%") {
                        if let Some((_, right)) = filter.split_once("-") {
                            filter = right.trim().to_string();
                        }
                    }
                    let start = item_string.to_lowercase().find(&filter.to_lowercase());
                    let start = match start {
                        Some(start) => start,
                        None => continue,
                    };
                    let end = filter.len() + start;
                    crossterm::queue!(
                        stdout,
                        MoveLeft((item_string.len() + (if start == 0 { 1 } else { 0 })) as u16),
                        MoveRight(start as u16),
                        SetAttribute(Attribute::Bold),
                        SetForegroundColor(Color::Black),
                        SetBackgroundColor(Color::Cyan),
                        Print(&item_string[start..end]),
                        SetAttribute(Attribute::Reset),
                        ResetColor,
                    )
                    .map_err(|e| UIError::Other(e.to_string()))?;
                }
            }

            crossterm::queue!(
                stdout,
                MoveUp(height as u16),
                MoveToColumn((prompt_len + filter_cursor) as u16)
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
                    KeyCode::Up | KeyCode::BackTab => {
                        selected_index -= 1;
                        direction = -1;
                        filter_mode = false
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        selected_index += 1;
                        direction = 1;
                        filter_mode = false
                    }
                    KeyCode::Enter => {
                        if result == 0 {
                            continue;
                        }
                        if let Some(index) = &item_slice.get(selected_index as usize)
                            && self.disabled_items.contains(&index)
                        {
                            continue;
                        }
                        if !self.multi_selection {
                            selected_indices.clear();
                            selected_indices.insert(
                                items[page_ranges[page_index].clone()][selected_index as usize],
                            );
                        } else {
                            if filter_mode {
                                filter_mode = false;
                                continue;
                            }
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
                    KeyCode::Char(c) => {
                        if key_event.modifiers == KeyModifiers::CONTROL {
                            if c == 'h' && self.filter_input {
                                Text::delete_word_back(&mut filter_cursor, &mut filter);
                                continue;
                            } else if c == 'c' {
                                render_prompt(
                                    &mut stdout,
                                    &self.message,
                                    &self.hint,
                                    "<interupted>",
                                    Color::Red,
                                    None,
                                    false,
                                )?;
                                cleanup_terminal(&mut stdout, true)?;
                            } else if c == 'a' && self.multi_selection {
                                for i in &items {
                                    selected_indices.insert(*i);
                                }
                                continue;
                            } else if c == 'd' && self.multi_selection {
                                for i in &items {
                                    selected_indices.remove(i);
                                }
                                continue;
                            }
                        }
                        if !self.filter_input {
                            continue;
                        }
                        if c == ' ' && !filter_mode && !items.is_empty() {
                            if !self.multi_selection {
                                continue;
                            } else if let Some(index) = &item_slice.get(selected_index as usize)
                                && self.disabled_items.contains(&index)
                            {
                                continue;
                            }
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
                        Text::type_char(&mut filter_cursor, &mut filter, c);
                    }
                    KeyCode::Backspace => {
                        if !self.filter_input {
                            continue;
                        }
                        filter_mode = true;
                        Text::delete_char_back(&mut filter_cursor, &mut filter);
                    }
                    KeyCode::Delete => {
                        if !self.filter_input {
                            continue;
                        }
                        filter_mode = true;
                        Text::delete_front(&mut filter_cursor, &mut filter, &key_event);
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
                        Text::move_left(&mut filter_cursor, &mut filter, &key_event);
                    }
                    KeyCode::Right => {
                        if !filter_mode || filter.is_empty() {
                            page_index = (page_index + 1) as usize;
                            if page_index > max_page_index {
                                page_index = 0;
                            }
                            continue;
                        }
                        Text::move_right(&mut filter_cursor, &mut filter, &key_event);
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

        render_prompt(
            &mut stdout,
            &self.message,
            &self.hint,
            &(if self.multi_selection {
                format!("Selected {} out of {}", selected_items.len(), list_size)
            } else {
                (self.formatter)(selected_items[0].1)
            }),
            Color::Green,
            Some(
                Attributes::default()
                    .with(Attribute::Bold)
                    .with(Attribute::Underlined),
            ),
            true,
        )?;

        cleanup_terminal(&mut stdout, false)?;

        let result = SelectResult {
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
            .with_disabled_items(vec![0, get_test_vec().len() - 1])
            .with_group_items(true);
        let mut first = get_test_vec();
        let mut second = first.split_off(7);

        // select.add_items(first.iter_mut().collect(), None);
        select.add_items(first.as_mut_slice(), Some("Separator desu:"));
        select.add_items(second.as_mut_slice(), Some("Separator 2 desu:"));

        let selection = select.prompt();

        match selection {
            Ok(result) => println!("You selected: {}", result.data()),
            Err(err) => println!("{:?}", err),
        }
    }

    fn test_multi_select() {
        let mut select = Select::<String>::new("Select a few things please", |i| i.clone())
            .with_multi_selection(true)
            .with_hint("Game please")
            .with_disabled_items(vec![0, get_test_vec().len() - 1])
            .with_page_size(6)
            .with_group_items(true);
        let mut first = get_test_vec();
        let mut second = first.split_off(7);

        select.add_items(first.as_mut_slice(), Some("Separator desu:"));
        select.add_items(second.as_mut_slice(), Some("Separator 2 desu:"));

        let selections = select.with_initial_selection(vec![0, 3, 4, 5, 6]).prompt();

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
