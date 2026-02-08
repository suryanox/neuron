use crate::db::{Database, Node};
use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Input,
    Search,
    Browse,
}

pub struct App {
    db: Database,
    pub mode: Mode,
    pub input: String,
    pub search: String,
    pub results: Vec<Node>,
    pub selected: usize,
    pub message: Option<String>,
    pub show_help: bool,
    pub quit: bool,
    clipboard: Option<Clipboard>,
}

impl App {
    pub fn new() -> Result<Self> {
        let db = Database::open()?;
        let results = db.get_recent(50)?;

        Ok(Self {
            db,
            mode: Mode::Input,
            input: String::new(),
            search: String::new(),
            results,
            selected: 0,
            message: None,
            show_help: false,
            quit: false,
            clipboard: Clipboard::new().ok(),
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        self.message = None;

        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => self.show_help = false,
                _ => {}
            }
            return Ok(());
        }

        match self.mode {
            Mode::Input => self.handle_input(key)?,
            Mode::Search => self.handle_search(key)?,
            Mode::Browse => self.handle_browse(key)?,
        }
        Ok(())
    }

    fn handle_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                if !self.input.trim().is_empty() {
                    let node = Node::new(self.input.trim().to_string(), None);
                    self.db.insert_node(&node)?;
                    self.message = Some("Saved!".to_string());
                    self.input.clear();
                    self.refresh_results()?;
                }
            }
            KeyCode::Tab => {
                self.mode = Mode::Search;
            }
            KeyCode::Esc => {
                if self.input.is_empty() {
                    self.mode = Mode::Browse;
                } else {
                    self.input.clear();
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_search(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                self.mode = Mode::Browse;
                self.selected = 0;
            }
            KeyCode::Tab => {
                self.mode = Mode::Input;
            }
            KeyCode::Esc => {
                if self.search.is_empty() {
                    self.mode = Mode::Browse;
                } else {
                    self.search.clear();
                    self.refresh_results()?;
                }
            }
            KeyCode::Backspace => {
                self.search.pop();
                self.refresh_results()?;
            }
            KeyCode::Char(c) => {
                self.search.push(c);
                self.refresh_results()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_browse(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                self.quit = true;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Tab => {
                self.mode = Mode::Input;
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < self.results.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            KeyCode::Char('y') => {
                self.copy_selected();
            }
            KeyCode::Char('d') => {
                self.delete_selected()?;
            }
            KeyCode::Esc => {
                self.mode = Mode::Input;
            }
            _ => {}
        }
        Ok(())
    }

    fn refresh_results(&mut self) -> Result<()> {
        self.results = if self.search.is_empty() {
            self.db.get_recent(50)?
        } else {
            self.db.search(&self.search)?
        };
        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
        Ok(())
    }

    fn copy_selected(&mut self) {
        if let Some(node) = self.results.get(self.selected) {
            if let Some(ref mut clip) = self.clipboard {
                match clip.set_text(&node.content) {
                    Ok(_) => self.message = Some("Copied!".to_string()),
                    Err(_) => self.message = Some("Failed to copy".to_string()),
                }
            } else {
                self.message = Some("Clipboard unavailable".to_string());
            }
        }
    }

    fn delete_selected(&mut self) -> Result<()> {
        if let Some(node) = self.results.get(self.selected) {
            let id = node.id.clone();
            self.db.delete_node(&id)?;
            self.message = Some("Deleted!".to_string());
            self.refresh_results()?;
        }
        Ok(())
    }
}
