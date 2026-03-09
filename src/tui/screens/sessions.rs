use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::tui::app::{App, Screen};
use crate::tui::theme;

pub fn render_list(f: &mut Frame, app: &App) {
    let Screen::Sessions {
        sessions, selected, ..
    } = &app.screen
    else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(5),    // sessions list
            Constraint::Length(3), // keybindings help
        ])
        .split(f.area());

    // Title bar
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " cortexmem ",
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0),
        ),
        Span::styled(" sessions ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            format!("({} total)", sessions.len()),
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

    // Sessions list
    if sessions.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No sessions found.",
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
        let list_items: Vec<ListItem> = sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let is_selected = i == *selected;

                let summary_preview = session
                    .summary
                    .as_deref()
                    .unwrap_or("(no summary)")
                    .chars()
                    .take(60)
                    .collect::<String>();

                let line = Line::from(vec![
                    Span::styled(
                        format!(" {:>4} ", session.id),
                        Style::default().fg(theme::SUBTEXT),
                    ),
                    Span::styled(
                        format!("[{}] ", session.project),
                        Style::default().fg(theme::BLUE),
                    ),
                    Span::styled(
                        format!("{} ", session.started_at),
                        Style::default().fg(theme::SUBTEXT),
                    ),
                    Span::styled(
                        summary_preview,
                        Style::default().fg(if is_selected {
                            theme::MAUVE
                        } else {
                            theme::TEXT
                        }),
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
                .title(Span::styled(" Sessions ", Style::default().fg(theme::BLUE)))
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

pub fn render_detail(f: &mut Frame, app: &App) {
    let Screen::SessionDetail { session, .. } = &app.screen else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(5),    // session info
            Constraint::Length(3), // keybindings help
        ])
        .split(f.area());

    // Title bar
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " cortexmem ",
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0),
        ),
        Span::styled(
            format!(" session #{} ", session.id),
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

    // Session info
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Project: ", Style::default().fg(theme::YELLOW)),
        Span::styled(&session.project, Style::default().fg(theme::BLUE)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Directory: ", Style::default().fg(theme::YELLOW)),
        Span::styled(&session.directory, Style::default().fg(theme::TEXT)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Started: ", Style::default().fg(theme::YELLOW)),
        Span::styled(&session.started_at, Style::default().fg(theme::SUBTEXT)),
    ]));

    let ended_str = session.ended_at.as_deref().unwrap_or("(active)");
    lines.push(Line::from(vec![
        Span::styled("Ended: ", Style::default().fg(theme::YELLOW)),
        Span::styled(
            ended_str,
            Style::default().fg(if session.ended_at.is_some() {
                theme::SUBTEXT
            } else {
                theme::GREEN
            }),
        ),
    ]));

    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "Summary:",
        Style::default().fg(theme::YELLOW),
    )));

    let summary_text = session.summary.as_deref().unwrap_or("(no summary)");
    for line in summary_text.lines() {
        lines.push(Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(theme::TEXT),
        )));
    }

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(
                    " Session Detail ",
                    Style::default().fg(theme::BLUE),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::SURFACE1))
                .style(Style::default().bg(theme::BASE)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(content, chunks[1]);

    // Keybindings help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Esc ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Back ", Style::default().fg(theme::SUBTEXT)),
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

pub fn handle_list_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.pop_screen();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Screen::Sessions {
                sessions, selected, ..
            } = &mut app.screen
                && !sessions.is_empty()
                && *selected < sessions.len() - 1
            {
                *selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Screen::Sessions { selected, .. } = &mut app.screen
                && *selected > 0
            {
                *selected -= 1;
            }
        }
        KeyCode::Enter => {
            if let Screen::Sessions {
                sessions, selected, ..
            } = &app.screen
                && let Some(session) = sessions.get(*selected)
            {
                let session = session.clone();
                app.push_screen(Screen::SessionDetail {
                    session,
                    observations: Vec::new(),
                    selected: 0,
                });
            }
        }
        _ => {}
    }
}

pub fn handle_detail_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.pop_screen();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        _ => {}
    }
}
