use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::tui::app::{App, Screen};
use crate::tui::theme;

pub fn render(f: &mut Frame, app: &App) {
    let Screen::Timeline {
        items, selected, ..
    } = &app.screen
    else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(5),    // items list
            Constraint::Length(3), // keybindings help
        ])
        .split(f.area());

    // Title bar
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " cortexmem ",
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0),
        ),
        Span::styled(" timeline ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            format!("({} items)", items.len()),
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

    // Items list
    if items.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No timeline items found.",
            Style::default().fg(theme::SUBTEXT),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::SURFACE1))
                .style(Style::default().bg(theme::BASE)),
        );
        f.render_widget(empty, chunks[1]);
    } else {
        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, obs)| {
                let is_selected = i == *selected;

                let line = Line::from(vec![
                    Span::styled(
                        format!(" {:>4} ", obs.id),
                        Style::default().fg(theme::SUBTEXT),
                    ),
                    Span::styled(
                        format!("[{}] ", obs.obs_type),
                        Style::default().fg(theme::YELLOW),
                    ),
                    Span::styled(
                        obs.title.clone(),
                        Style::default().fg(if is_selected {
                            theme::MAUVE
                        } else {
                            theme::TEXT
                        }),
                    ),
                    Span::styled(
                        format!("  {}", obs.created_at),
                        Style::default().fg(theme::SUBTEXT),
                    ),
                    Span::styled(
                        format!("  [{}]", obs.tier),
                        Style::default().fg(theme::LAVENDER),
                    ),
                ]);

                let style = if is_selected {
                    Style::default().bg(theme::SURFACE0)
                } else {
                    Style::default().bg(theme::BASE)
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(list_items).block(
            Block::default()
                .title(Span::styled(
                    " Observations ",
                    Style::default().fg(theme::BLUE),
                ))
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
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Screen::Timeline {
                items, selected, ..
            } = &mut app.screen
                && !items.is_empty()
                && *selected < items.len() - 1
            {
                *selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Screen::Timeline { selected, .. } = &mut app.screen
                && *selected > 0
            {
                *selected -= 1;
            }
        }
        KeyCode::Enter => {
            if let Screen::Timeline {
                items, selected, ..
            } = &app.screen
                && let Some(obs) = items.get(*selected)
            {
                let id = obs.id;
                if let Ok(Some(full_obs)) = app.server.call_get(id) {
                    app.push_screen(Screen::ObservationDetail {
                        obs: Box::new(full_obs),
                        scroll: 0,
                    });
                }
            }
        }
        _ => {}
    }
}
