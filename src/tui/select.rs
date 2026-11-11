use std::{
    cmp::Ordering,
    collections::HashSet,
    io::{Write, stdout},
    ops::Range,
};

use crate::tui::{text::*, util::*};

use crossterm::{
    cursor::{self, MoveToColumn, MoveToNextLine, MoveUp},
    event::{self, Event, KeyCode, KeyModifiers},
    style::{
        Attribute::{self},
        Attributes, Color, Print, ResetColor, SetAttribute, SetAttributes, SetForegroundColor,
        Stylize,
    },
    terminal::{self, ScrollUp},
};
use fuzzy_matcher::{
    FuzzyMatcher,
    skim::{SkimMatcherV2, SkimScoreConfig},
};

#[derive(Clone)]
struct SelectItem<T> {
    data: T,
    group: Option<String>,
    collapsed: bool,
}

pub struct Select<'a, T> {
    message: &'a str,
    items: Vec<SelectItem<&'a mut T>>,
    formatter: fn(&T) -> String,
    finished_formatter: fn(&T) -> String,
    sorter: Option<fn(&T, &T) -> Ordering>,
    selected_color: Color,
    group_items: bool,
    collapsed_groups: Vec<String>,
    disabled_items: Vec<usize>,
    hint: Option<String>,
    page_size: Option<usize>,
    initial_selections: Vec<usize>,
    initial_position: usize,
    auto_select: Option<usize>,
    filter_input: bool,
    initial_filter: String,
    multi_selection: bool,
    hybrid: bool,
    select_on_enter: bool,
}

pub struct SelectResult<'a, T> {
    pub selected: Vec<(usize, &'a mut T)>,
    pub unselected: Vec<(usize, &'a mut T)>,
}

impl<'a, T> SelectResult<'a, T> {
    pub fn nth_data(&'a self, index: usize) -> &'a T {
        self.selected[index].1
    }
    pub fn nth_data_mut(&'a mut self, index: usize) -> &'a mut T {
        self.selected[index].1
    }
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
            finished_formatter: formatter,
            sorter: None,
            selected_color: Color::Green,
            auto_select: None,
            hint: None,
            group_items: false,
            collapsed_groups: Vec::new(),
            initial_position: 0,
            disabled_items: Vec::new(),
            page_size: None,
            initial_selections: Vec::new(),
            filter_input: true,
            initial_filter: String::new(),
            multi_selection: false,
            hybrid: false,
            select_on_enter: false,
        }
    }

    pub fn with_initial_position(mut self, initial_position: usize) -> Self {
        self.initial_position = initial_position;
        self
    }

    pub fn with_auto_select(mut self, auto_select: usize) -> Self {
        self.auto_select = Some(auto_select);
        self
    }

    pub fn with_sorter(mut self, sorter: fn(&T, &T) -> Ordering) -> Self {
        self.sorter = Some(sorter);
        self
    }

    pub fn with_collapsed_groups(mut self, collaped_groups: Vec<String>) -> Self {
        self.collapsed_groups = collaped_groups;
        self
    }

    pub fn with_finished_formatter(mut self, finished_formatter: fn(&T) -> String) -> Self {
        self.finished_formatter = finished_formatter;
        self
    }

    pub fn with_select_on_enter(mut self, select_on_enter: bool) -> Self {
        self.select_on_enter = select_on_enter;
        self
    }

    pub fn with_multi_selection(mut self, multi_selection: bool) -> Self {
        self.multi_selection = multi_selection;
        self
    }

    pub fn with_hybrid(mut self, hybrid: bool) -> Self {
        self.hybrid = hybrid;
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
                group: group.map(|g| g.to_string()),
                collapsed: false,
            };
            self.items.push(select_item)
        }
    }

    pub fn add_item(&mut self, item: &'a mut T, group: Option<&str>) {
        self.items.push(SelectItem {
            data: item,
            group: group.map(|g| g.to_string()),
            collapsed: false,
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

    fn match_item(&self, item: &T, query: &str) -> bool {
        let item = (self.formatter)(item);
        if let Some(fzf) = SkimMatcherV2::default()
            .score_config(SkimScoreConfig {
                score_match: 16,
                gap_start: -3,
                gap_extension: -1,
                bonus_first_char_multiplier: 2,
                bonus_head: 8,
                bonus_break: 7,
                bonus_camel: 6,
                bonus_consecutive: 4,
                penalty_case_mismatch: -2,
            })
            .fuzzy_match(&item, &query)
        {
            if fzf >= 10 {
                return true;
            }
        }

        false
    }

    fn match_filter(&self, item: &SelectItem<&'a mut T>, filter: &str) -> bool {
        match &item.group {
            Some(group) if filter.starts_with("%") => {
                let stripped_filter = filter.strip_prefix("%").unwrap();
                let (mut group_filter, mut item_filter) = stripped_filter
                    .split_once("-")
                    .unwrap_or((stripped_filter, ""));
                group_filter = group_filter.trim();
                item_filter = item_filter.trim();
                if group
                    .to_lowercase()
                    .replace(" ", "")
                    .contains(&group_filter.to_lowercase().replace(" ", ""))
                {
                    if item_filter.is_empty() {
                        return true;
                    }
                    return self.match_item(&item.data, &item_filter.to_lowercase());
                }
            }
            _ => return self.match_item(&item.data, &filter.to_lowercase()),
        }
        false
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

    fn get_filtered_items(&mut self) -> Vec<usize> {
        let mut filtered_items = Vec::new();
        let mut group = None;
        for (i, item) in self.items.iter_mut().enumerate() {
            if let Some(item_group) = &item.group {
                if self.collapsed_groups.contains(&item_group) {
                    item.collapsed = true;
                    if item.group == group {
                        continue;
                    }
                }
            }
            if item.group != group {
                group = item.group.clone()
            }
            filtered_items.push(i);
        }
        filtered_items
    }

    fn sort_items(&self, items: &mut Vec<usize>, sorter: fn(&T, &T) -> Ordering) {
        let mut group_tags = items
            .chunk_by(|a, b| self.items[*a].group == self.items[*b].group)
            .map(|i| i.to_vec())
            .collect::<Vec<_>>();
        for group in group_tags.iter_mut() {
            group.sort_by(|a, b| sorter(self.items[*a].data, self.items[*b].data));
        }
        *items = group_tags.concat()
    }

    fn filter_items(&self, filtered_items: &[usize], filter: &str) -> Vec<usize> {
        let mut result = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let match_filter = self.match_filter(item, &filter);
                if (filtered_items.contains(&i) && filter.is_empty())
                    || (!filter.is_empty() && match_filter)
                {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        if let Some(sorter) = self.sorter {
            self.sort_items(&mut result, sorter);
        }

        result
    }

    pub fn prompt(mut self) -> Result<SelectResult<'a, T>, UIError> {
        let mut stdout = stdout();
        let mut filter = self.initial_filter.clone();
        let mut filter_cursor = 0;
        let mut filter_mode = false;
        let mut page_index = 0;
        let mut page_ranges: Vec<Range<usize>> = Vec::new();
        let base_group_count = self.count_group(self.items.iter().map(|i| i).collect());
        let mut direction = 0;
        let mut changed = false;

        let filtered_items = self.get_filtered_items();

        let mut items = self.filter_items(&filtered_items, &filter);

        self.calculate_page_ranges(&self.page_size, &items, &mut page_ranges);

        let mut max_page_index;

        crossterm::terminal::enable_raw_mode().map_err(|e| UIError::Terminal(e.to_string()))?;
        crossterm::execute!(stdout, cursor::SetCursorStyle::BlinkingBar)
            .map_err(|e| UIError::Terminal(e.to_string()))?;

        let mut selected_index: i64 = self.initial_position as i64;
        let mut selected_indices: HashSet<usize> = if self.multi_selection {
            self.initial_selections.iter().cloned().collect()
        } else {
            if !self.initial_selections.is_empty() {
                for (i, range) in page_ranges.iter().enumerate() {
                    match range
                        .clone()
                        .into_iter()
                        .position(|i| i == self.initial_selections[0])
                    {
                        Some(ii) => {
                            page_index = i;
                            selected_index = ii as i64;
                        }
                        None => {}
                    }
                }
            }
            HashSet::new()
        };

        let toggling_hint = if self.multi_selection {
            "| [Space] Toggle "
        } else {
            ""
        };

        let mut save_state = None;

        loop {
            if let Some(auto_select) = self.auto_select {
                selected_indices.clear();
                selected_indices.insert(auto_select);
                break;
            }

            items = self.filter_items(&filtered_items, &filter);

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

            let selected_item_index = *items
                .get(page_ranges.get(page_index).unwrap_or(&(0..0)).clone())
                .unwrap_or(&[])
                .get(selected_index as usize)
                .unwrap_or(&0);

            direction = 0;

            let (_, cy) = cursor::position().map_err(|e| UIError::Terminal(e.to_string()))?;
            let (_, th) = terminal::size().map_err(|e| UIError::Terminal(e.to_string()))?;

            let space_needed = (cy as i32
                + (self
                    .page_size
                    .unwrap_or(self.items.len() + base_group_count)
                    + 4) as i32
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
                        + 3) as u16
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
                        + 2) as u16
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
                let toggled_str = if toggled { "✓" } else { " " };
                let color = if item.collapsed && filter.is_empty() {
                    Color::Cyan
                } else if selected {
                    Color::Yellow
                } else if toggled {
                    self.selected_color
                } else {
                    Color::Reset
                };
                let attributes = if item.collapsed && filter.is_empty() {
                    if selected {
                        Attributes::default()
                            .with(Attribute::Bold)
                            .with(Attribute::Italic)
                    } else {
                        Attributes::default()
                            .with(Attribute::Bold)
                            .with(Attribute::Italic)
                            .with(Attribute::Dim)
                    }
                } else if selected {
                    Attributes::default().with(Attribute::Bold)
                } else {
                    Attributes::none()
                };
                let disabled = self.disabled_items.contains(&item_i);
                let show_check_box = self.multi_selection
                    && (!self.hybrid || (self.hybrid && selected_indices.len() > 0));
                let item_string_formatted = if item.collapsed && filter.is_empty() {
                    let count = self.items.iter().filter(|it| it.group == item.group).count();
                    let selected_count = self.items.iter().enumerate().filter(|(i, it)| it.group == item.group && selected_indices.contains(&i)).count();
                    format!(
                        "{}[{} collapsed items]",
                        if show_check_box {
                            format!("[{}/{}] ", selected_count, count)
                        } else {
                            String::new()
                        },
                        self.items.iter().filter(|i| i.group == item.group).count()
                    )
                } else {
                    format!(
                        "{}{}",
                        if show_check_box {
                            format!("[{}] ", toggled_str)
                        } else {
                            String::new()
                        },
                        item_string
                    )
                };
                let item_string_formatted = if disabled {
                    item_string_formatted.reset().dim().crossed_out()
                } else {
                    item_string_formatted.stylize()
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
            }

            crossterm::queue!(
                stdout,
                MoveUp((height + 1) as u16),
                MoveToColumn((prompt_len + filter_cursor) as u16)
            )
            .map_err(|e| UIError::Other(e.to_string()))?;

            // if items.is_empty() {
            //     crossterm::queue!(stdout, MoveDown(height as u16))
            //         .map_err(|e| UIError::Other(e.to_string()))?;
            // }

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
                        if filter.is_empty()
                            && (!self.multi_selection
                                || ((self.hybrid && selected_indices.is_empty())
                                    || (self.select_on_enter && !changed)))
                        {
                            let item = &self.items[selected_item_index];
                            if item.collapsed {
                                match &item.group {
                                    Some(group) => {
                                        filter = format!("%{} - ", group);
                                        filter_cursor = filter.len();
                                        save_state = Some((page_index, selected_index));
                                        selected_index = 0;
                                        page_index = 0;
                                    }
                                    None => {}
                                };
                                continue;
                            }
                        }
                        if let Some(index) = &item_slice.get(selected_index as usize)
                            && self.disabled_items.contains(&index)
                        {
                            continue;
                        }
                        if filter_mode {
                            filter_mode = false;
                            continue;
                        }
                        if self.multi_selection && !(self.hybrid && selected_indices.is_empty()) {
                            if self.select_on_enter && !changed {
                                if selected_indices.contains(&selected_item_index) {
                                    selected_indices.remove(&selected_item_index);
                                } else {
                                    selected_indices.insert(selected_item_index);
                                }
                            }
                        } else if !self.multi_selection || (self.hybrid && selected_indices.is_empty()) {
                            selected_indices.clear();
                            selected_indices.insert(selected_item_index);
                        }
                        break;
                    }
                    KeyCode::Esc => {
                        if filter_mode {
                            filter_mode = false;
                        }
                        if !filter.is_empty() {
                            filter.clear();
                            filter_cursor = 0;
                            match save_state {
                                Some((i, si)) => {
                                    page_index = i;
                                    selected_index = si;
                                    save_state = None
                                }
                                None => {}
                            }
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
                            if !self.filter_input {
                                continue;
                            }
                            if c == 'h' {
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
                                    let item = &self.items[*i];
                                    if item.collapsed {
                                        for (ii, group_item) in self.items.iter().enumerate() {
                                            if group_item.group == item.group {
                                                selected_indices.insert(ii);
                                            }
                                        }
                                        continue;
                                    }
                                    selected_indices.insert(*i);
                                }
                                continue;
                            } else if c == 'd' && self.multi_selection {
                                for i in &items {
                                    let item = &self.items[*i];
                                    if item.collapsed {
                                        for (ii, group_item) in self.items.iter().enumerate() {
                                            if group_item.group == item.group {
                                                selected_indices.remove(&ii);
                                            }
                                        }
                                        continue;
                                    }
                                    selected_indices.remove(i);
                                }
                                continue;
                            } else if c == 'z' && self.multi_selection {
                                selected_indices = self.initial_selections.iter().cloned().collect()
                            }
                        }
                        if c == ' ' && !filter_mode && !items.is_empty() {
                            if filter.is_empty() {
                                let item = &self.items[selected_item_index];
                                if item.collapsed {
                                    match &item.group {
                                        Some(group) => {
                                            filter = format!("%{} - ", group);
                                            filter_cursor = filter.len();
                                            save_state = Some((page_index, selected_index));
                                            selected_index = 0;
                                            page_index = 0;
                                        }
                                        None => {}
                                    };
                                    continue;
                                }
                            }
                            if !self.multi_selection {
                                continue;
                            } else if let Some(index) = &item_slice.get(selected_index as usize)
                                && self.disabled_items.contains(&index)
                            {
                                continue;
                            }
                            let to_toggle = selected_item_index;
                            changed = true;
                            if selected_indices.contains(&to_toggle) {
                                selected_indices.remove(&to_toggle);
                            } else {
                                selected_indices.insert(to_toggle);
                            }
                            continue;
                        }
                        if !filter_mode {
                            if c == 'k' {
                                selected_index += 1;
                                direction = 1;
                            } else if c == 'K' {
                                selected_index += 3;
                                direction = 1;
                            } else if c == 'i' {
                                selected_index -= 1;
                                direction = -1;
                            } else if c == 'I' {
                                selected_index -= 3;
                                direction = 1;
                            } else if c == 'j' {
                                if page_index as i64 - 1 < 0 {
                                    page_index = max_page_index;
                                    continue;
                                }
                                page_index = (page_index as i64 - 1) as usize;
                            } else if c == 'l' {
                                page_index = (page_index + 1) as usize;
                                if page_index > max_page_index {
                                    page_index = 0;
                                }
                            } else if c == '/' && self.filter_input {
                                save_state = None;
                                filter_mode = true;
                            }
                            continue;
                        } else {
                        }
                        Text::type_char(&mut filter_cursor, &mut filter, c);
                    }
                    KeyCode::Backspace => {
                        if !filter_mode {
                            continue;
                        }
                        Text::delete_char_back(&mut filter_cursor, &mut filter);
                    }
                    KeyCode::Delete => {
                        if !filter_mode {
                            continue;
                        }
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
            &(if self.multi_selection && selected_indices.len() != 1 {
                format!("Selected {} out of {}", selected_items.len(), list_size)
            } else {
                (self.finished_formatter)(selected_items[0].1)
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
