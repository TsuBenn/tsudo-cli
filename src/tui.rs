pub mod confirm;
pub mod text;
pub mod select;
mod util;

pub use confirm::Confirm;
pub use text::Text;
pub use select::Select;
pub use util::UIError;
pub use util::clean_line;
pub use util::clean_lines;
pub use util::clean_line_once;

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
            .with_validation(|input| {
                if !get_test_vec().contains(&input.to_string()) {
                    (false, Some("Unavailable Game!".to_string()))
                } else {
                    (true, None)
                }
            })
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
            .with_collapsed_groups(vec!["Separator 2 desu:".to_owned()])
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
