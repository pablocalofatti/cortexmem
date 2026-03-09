use std::sync::Arc;

use crate::db::{Observation, Session};
use crate::mcp::CortexMemServer;
use crate::search::SearchResult;

#[allow(dead_code)]
pub enum Screen {
    Dashboard,
    Search {
        query: String,
        cursor: usize,
    },
    SearchResults {
        query: String,
        results: Vec<SearchResult>,
        selected: usize,
    },
    ObservationDetail {
        obs: Box<Observation>,
        scroll: u16,
    },
    Timeline {
        center: i64,
        items: Vec<Observation>,
        selected: usize,
    },
    Sessions {
        sessions: Vec<Session>,
        selected: usize,
    },
    SessionDetail {
        session: Session,
        observations: Vec<Observation>,
        selected: usize,
    },
}

pub struct App {
    pub screen: Screen,
    pub server: Arc<CortexMemServer>,
    pub should_quit: bool,
    pub screen_stack: Vec<Screen>,
}

impl App {
    pub fn new(server: Arc<CortexMemServer>) -> Self {
        Self {
            screen: Screen::Dashboard,
            server,
            should_quit: false,
            screen_stack: Vec::new(),
        }
    }

    pub fn push_screen(&mut self, screen: Screen) {
        let prev = std::mem::replace(&mut self.screen, screen);
        self.screen_stack.push(prev);
    }

    pub fn pop_screen(&mut self) {
        if let Some(prev) = self.screen_stack.pop() {
            self.screen = prev;
        }
    }
}
