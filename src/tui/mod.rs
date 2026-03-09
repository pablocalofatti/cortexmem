pub mod app;
pub mod screens;
pub mod theme;

use std::io;
use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::mcp::CortexMemServer;

use self::app::{App, Screen};

pub fn run(server: Arc<CortexMemServer>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(server);

    loop {
        terminal.draw(|f| render(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_input(&mut app, key);
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn render(f: &mut ratatui::Frame, app: &App) {
    match &app.screen {
        Screen::Dashboard => screens::dashboard::render(f, app),
        Screen::Search { .. } => screens::search::render_input(f, app),
        Screen::SearchResults { .. } => screens::search::render_results(f, app),
        Screen::ObservationDetail { .. } => screens::detail::render(f, app),
        Screen::Timeline { .. } => screens::timeline::render(f, app),
        Screen::Sessions { .. } => screens::sessions::render_list(f, app),
        Screen::SessionDetail { .. } => screens::sessions::render_detail(f, app),
    }
}

fn handle_input(app: &mut App, key: crossterm::event::KeyEvent) {
    match &app.screen {
        Screen::Dashboard => handle_dashboard_input(app, key),
        _ => {
            if key.code == KeyCode::Esc {
                app.pop_screen();
            }
        }
    }
}

fn handle_dashboard_input(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('s') | KeyCode::Char('/') => {
            app.push_screen(Screen::Search {
                query: String::new(),
                cursor: 0,
            });
        }
        KeyCode::Char('n') => {
            app.push_screen(Screen::Sessions {
                sessions: Vec::new(),
                selected: 0,
            });
        }
        _ => {}
    }
}
