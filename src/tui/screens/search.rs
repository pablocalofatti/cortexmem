use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::tui::app::{App, Screen};
use crate::tui::theme;

pub fn render_input(f: &mut Frame, app: &App) {
    let Screen::Search { query, cursor } = &app.screen else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Length(3), // search input
            Constraint::Min(1),    // hint area
            Constraint::Length(3), // keybindings help
        ])
        .split(f.area());

    // Title bar
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " cortexmem ",
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0),
        ),
        Span::styled(" search ", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme::SURFACE1)),
    )
    .style(Style::default().bg(theme::BASE));
    f.render_widget(title, chunks[0]);

    // Search input
    let input_block = Block::default()
        .title(Span::styled(" Query ", Style::default().fg(theme::BLUE)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::LAVENDER))
        .style(Style::default().bg(theme::BASE));

    let input = Paragraph::new(Line::from(vec![Span::styled(
        query.as_str(),
        Style::default().fg(theme::TEXT),
    )]))
    .block(input_block);
    f.render_widget(input, chunks[1]);

    // Place cursor
    let cursor_x = chunks[1].x + 1 + *cursor as u16;
    let cursor_y = chunks[1].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));

    // Hint area
    let hint = Paragraph::new(Line::from(vec![Span::styled(
        "Type your search query and press Enter to search.",
        Style::default().fg(theme::SUBTEXT),
    )]))
    .style(Style::default().bg(theme::BASE));
    f.render_widget(hint, chunks[2]);

    // Keybindings help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Esc ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Back ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" Enter ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Search ", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::SURFACE1)),
    )
    .style(Style::default().bg(theme::BASE));
    f.render_widget(help, chunks[3]);
}

pub fn render_results(f: &mut Frame, app: &App) {
    let Screen::SearchResults {
        query,
        results,
        selected,
    } = &app.screen
    else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(5),    // results list
            Constraint::Length(3), // keybindings help
        ])
        .split(f.area());

    // Title bar with query
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " cortexmem ",
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0),
        ),
        Span::styled(" results for ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(format!("\"{}\"", query), Style::default().fg(theme::YELLOW)),
        Span::styled(
            format!(" ({} found)", results.len()),
            Style::default().fg(theme::SUBTEXT),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme::SURFACE1)),
    )
    .style(Style::default().bg(theme::BASE));
    f.render_widget(title, chunks[0]);

    // Results list
    if results.is_empty() {
        let empty = Paragraph::new(Line::from(vec![Span::styled(
            "No results found.",
            Style::default().fg(theme::SUBTEXT),
        )]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::SURFACE1))
                .style(Style::default().bg(theme::BASE)),
        );
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = results
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let is_selected = i == *selected;
                let score_str = format!("{:.2}", result.score);
                let concepts_str = result
                    .concepts
                    .as_ref()
                    .map(|c| c.join(", "))
                    .unwrap_or_default();

                let line = Line::from(vec![
                    Span::styled(
                        format!(" {:>4} ", result.id),
                        Style::default().fg(theme::SUBTEXT),
                    ),
                    Span::styled(
                        format!("[{}] ", result.obs_type),
                        Style::default().fg(theme::YELLOW),
                    ),
                    Span::styled(
                        result.title.clone(),
                        Style::default().fg(if is_selected {
                            theme::MAUVE
                        } else {
                            theme::TEXT
                        }),
                    ),
                    Span::styled(
                        format!("  ({})", score_str),
                        Style::default().fg(theme::GREEN),
                    ),
                    if !concepts_str.is_empty() {
                        Span::styled(
                            format!("  [{}]", concepts_str),
                            Style::default().fg(theme::BLUE),
                        )
                    } else {
                        Span::raw("")
                    },
                ]);

                let style = if is_selected {
                    Style::default().bg(theme::SURFACE0)
                } else {
                    Style::default().bg(theme::BASE)
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(" Results ", Style::default().fg(theme::BLUE)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::SURFACE1))
                .style(Style::default().bg(theme::BASE)),
        );
        f.render_widget(list, chunks[1]);
    }

    // Keybindings help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Esc ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Back ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            " j/k/\u{2191}/\u{2193} ",
            Style::default().fg(theme::BASE).bg(theme::MAUVE),
        ),
        Span::styled(" Navigate ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" Enter ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Open ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" q ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Quit ", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::SURFACE1)),
    )
    .style(Style::default().bg(theme::BASE));
    f.render_widget(help, chunks[2]);
}

pub fn handle_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.pop_screen();
        }
        KeyCode::Enter => {
            if let Screen::Search { query, .. } = &app.screen
                && !query.is_empty()
            {
                let query_owned = query.clone();
                let results = app.server.call_search(&query_owned, None, None, None, None);
                app.screen = Screen::SearchResults {
                    query: query_owned,
                    results,
                    selected: 0,
                };
            }
        }
        KeyCode::Backspace => {
            if let Screen::Search { query, cursor } = &mut app.screen
                && *cursor > 0
            {
                query.remove(*cursor - 1);
                *cursor -= 1;
            }
        }
        KeyCode::Left => {
            if let Screen::Search { cursor, .. } = &mut app.screen
                && *cursor > 0
            {
                *cursor -= 1;
            }
        }
        KeyCode::Right => {
            if let Screen::Search { query, cursor } = &mut app.screen
                && *cursor < query.len()
            {
                *cursor += 1;
            }
        }
        KeyCode::Home => {
            if let Screen::Search { cursor, .. } = &mut app.screen {
                *cursor = 0;
            }
        }
        KeyCode::End => {
            if let Screen::Search { query, cursor } = &mut app.screen {
                *cursor = query.len();
            }
        }
        KeyCode::Char(c) => {
            if let Screen::Search { query, cursor } = &mut app.screen {
                query.insert(*cursor, c);
                *cursor += 1;
            }
        }
        _ => {}
    }
}

pub fn handle_results_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.pop_screen();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Screen::SearchResults {
                results, selected, ..
            } = &mut app.screen
                && !results.is_empty()
                && *selected < results.len() - 1
            {
                *selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Screen::SearchResults { selected, .. } = &mut app.screen
                && *selected > 0
            {
                *selected -= 1;
            }
        }
        KeyCode::Enter => {
            if let Screen::SearchResults {
                results, selected, ..
            } = &app.screen
                && let Some(result) = results.get(*selected)
            {
                let id = result.id;
                if let Ok(Some(obs)) = app.server.call_get(id) {
                    app.push_screen(Screen::ObservationDetail {
                        obs: Box::new(obs),
                        scroll: 0,
                    });
                }
            }
        }
        _ => {}
    }
}
