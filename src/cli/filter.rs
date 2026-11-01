use crossterm::style::Stylize;
use fuzzy_matcher::{
    FuzzyMatcher,
    skim::{SkimMatcherV2, SkimScoreConfig},
};
use strsim::damerau_levenshtein;

use crate::{
    cli::util::handle_ui_error,
    todo::{TodoFindError, TodoList},
};
use tsudo::tui::*;

fn match_tags(tag: &str, query: &str) -> bool {
    let tag = tag.trim().to_lowercase();

    if query == tag {
        return true;
    }

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
        .fuzzy_match(&tag, query)
    {
        if fzf >= 10 {
            return true;
        }
    }

    let sub = query.to_lowercase().contains(&tag) || tag.to_lowercase().contains(query);
    if sub {
        return true;
    }

    let lv = damerau_levenshtein(&tag, query);
    if lv <= 2 {
        return true;
    }

    false
}

pub fn todo_filter(
    todo_list: Result<TodoList, TodoFindError>,
    input_tag: String,
) -> Result<(), i32> {
    if let Ok(mut todo_list) = todo_list {
        let mut tags = todo_list.get_tags();
        if !input_tag.is_empty() {
            tags = tags
                .into_iter()
                .filter(|t| match_tags(t, &input_tag.trim().to_lowercase()))
                .collect();
        }
        let tag;
        if tags.len() == 1 {
            tag = tags[0].clone()
        } else {
            if tags.is_empty() {
                eprintln!(
                    "\n{}\n",
                    format!("  No Tag of '{}' found.", input_tag).bold().red()
                );
                tags = todo_list.get_tags();
            }
            let mut tag_select = Select::<String>::new("Enter Tag filter", |i| {
                if i.is_empty() {
                    "__untagged__".to_string()
                } else {
                    i.clone()
                }
            });
            tag_select.add_items(tags.as_mut_slice(), None);
            let result = tag_select
                .prompt()
                .map_err(|err| handle_ui_error(err, ""))?;
            tag = result.selected[0].1.clone();
        }
        todo_list.config.show_completed_range = None;
        todo_list.config.hide_completed_groups = false;
        if todo_list.config.sort_by_completion {
            todo_list.items.sort_by_key(|i| i.done);
        }
        todo_list.items.retain(|i| i.tag == tag);
        todo_list.print();
    }

    Ok(())
}
