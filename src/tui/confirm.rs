use std::io::{Write, stdout};

use crossterm::{
    cursor::{self},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    style::{
        Attribute::{self},
        Attributes, Color,
    },
};

use crate::tui::util::*;

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

            stdout.flush().map_err(|e| UIError::Other(e.to_string()))?;

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
