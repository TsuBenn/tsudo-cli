use std::collections::HashSet;

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
    if tag.is_empty() {
        return false;
    }
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
    input_tags: Vec<String>,
) -> Result<(), i32> {
    if let Ok(mut todo_list) = todo_list {
        let mut tags = todo_list.get_tags();
        let mut hit = HashSet::new();
        let mut misses = HashSet::new();
        if !input_tags.is_empty() {
            for input in input_tags {
                let matched_tags: Vec<usize> = tags
                    .iter()
                    .enumerate()
                    .filter_map(|(i, tag)| {
                        if match_tags(&tag, &input) {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect();
                if matched_tags.is_empty() {
                    continue;
                }
                if matched_tags.len() == 1 {
                    hit.insert(*matched_tags.get(0).unwrap());
                } else {
                    for miss in matched_tags {
                        misses.insert(miss);
                    }
                }
            }
        }
        let tag: Vec<String>;
        if !hit.is_empty() && misses.is_empty() {
            tag = tags
                .into_iter()
                .enumerate()
                .filter_map(|(i, item)| if hit.contains(&i) { Some(item) } else { None })
                .collect();
        } else if hit.is_empty() && misses.is_empty() {
            eprintln!("\n  No Tags of found.\n",);
            tag = tags;
        } else {
            let matched_tags: Vec<(usize, String)> = tags.into_iter().enumerate().filter(|(i,_)| hit.contains(&i) || misses.contains(&i)).collect();
            let mut tag_select = Select::<String>::new("Enter Tag filter", |i| {
                if i.is_empty() {
                    "__untagged__".to_string()
                } else {
                    i.clone()
                }
            })
            .with_multi_selection(true)
            .with_hybrid(true)
            .with_initial_selection(matched_tags.iter().filter_map(|(i, _)| if hit.contains(&i) {Some(i.clone())} else {None}).collect());
            tags = matched_tags.into_iter().map(|(_, item)| item).collect();
            tag_select.add_items(&mut tags, None);
            let result = tag_select
                .prompt()
                .map_err(|err| handle_ui_error(err, ""))?;
            tag = result
                .selected
                .into_iter()
                .map(|(_, tag)| tag.clone())
                .collect();
        }
        todo_list.config.show_completed_range = None;
        todo_list.config.hide_completed_groups = false;
        if todo_list.config.sort_by_completion {
            todo_list.items.sort_by_key(|i| i.done);
        }
        todo_list.items.retain(|i| tag.contains(&i.tag));
        todo_list.print();
    }

    Ok(())
}
