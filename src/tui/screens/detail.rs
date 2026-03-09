use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::app::{App, Screen};
use crate::tui::theme;

pub fn render(f: &mut Frame, app: &App) {
    let Screen::ObservationDetail { obs, scroll } = &app.screen else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(5),    // content area
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
            format!(" observation #{} ", obs.id),
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

    // Build content lines
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(theme::YELLOW)),
            Span::styled(&obs.title, Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(theme::YELLOW)),
            Span::styled(&obs.obs_type, Style::default().fg(theme::LAVENDER)),
            Span::styled("  Tier: ", Style::default().fg(theme::YELLOW)),
            Span::styled(&obs.tier, Style::default().fg(theme::LAVENDER)),
            Span::styled("  Scope: ", Style::default().fg(theme::YELLOW)),
            Span::styled(&obs.scope, Style::default().fg(theme::LAVENDER)),
        ]),
        Line::from(vec![
            Span::styled("Project: ", Style::default().fg(theme::YELLOW)),
            Span::styled(&obs.project, Style::default().fg(theme::BLUE)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Content:", Style::default().fg(theme::YELLOW))),
    ];

    for content_line in obs.content.lines() {
        lines.push(Line::from(Span::styled(
            content_line.to_string(),
            Style::default().fg(theme::TEXT),
        )));
    }

    lines.push(Line::from(""));

    if let Some(concepts) = &obs.concepts
        && !concepts.is_empty()
    {
        lines.push(Line::from(vec![
            Span::styled("Concepts: ", Style::default().fg(theme::YELLOW)),
            Span::styled(concepts.join(", "), Style::default().fg(theme::GREEN)),
        ]));
    }

    if let Some(facts) = &obs.facts
        && !facts.is_empty()
    {
        lines.push(Line::from(vec![
            Span::styled("Facts: ", Style::default().fg(theme::YELLOW)),
            Span::styled(facts.join(", "), Style::default().fg(theme::GREEN)),
        ]));
    }

    if let Some(files) = &obs.files
        && !files.is_empty()
    {
        lines.push(Line::from(vec![
            Span::styled("Files: ", Style::default().fg(theme::YELLOW)),
            Span::styled(files.join(", "), Style::default().fg(theme::BLUE)),
        ]));
    }

    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("Created: ", Style::default().fg(theme::YELLOW)),
        Span::styled(&obs.created_at, Style::default().fg(theme::SUBTEXT)),
        Span::styled("  Updated: ", Style::default().fg(theme::YELLOW)),
        Span::styled(&obs.updated_at, Style::default().fg(theme::SUBTEXT)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Access count: ", Style::default().fg(theme::YELLOW)),
        Span::styled(
            obs.access_count.to_string(),
            Style::default().fg(theme::TEXT),
        ),
        Span::styled("  Revisions: ", Style::default().fg(theme::YELLOW)),
        Span::styled(
            obs.revision_count.to_string(),
            Style::default().fg(theme::TEXT),
        ),
    ]));

    if let Some(topic) = &obs.topic_key {
        lines.push(Line::from(vec![
            Span::styled("Topic: ", Style::default().fg(theme::YELLOW)),
            Span::styled(topic, Style::default().fg(theme::MAUVE)),
        ]));
    }

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" Detail ", Style::default().fg(theme::BLUE)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::SURFACE1))
                .style(Style::default().bg(theme::BASE)),
        )
        .wrap(Wrap { trim: false })
        .scroll((*scroll, 0));
    f.render_widget(content, chunks[1]);

    // Keybindings help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Esc ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Back ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(
            " j/k/\u{2191}/\u{2193} ",
            Style::default().fg(theme::BASE).bg(theme::MAUVE),
        ),
        Span::styled(" Scroll ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" t ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Timeline ", Style::default().fg(theme::SUBTEXT)),
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
            if let Screen::ObservationDetail { scroll, .. } = &mut app.screen {
                *scroll = scroll.saturating_add(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Screen::ObservationDetail { scroll, .. } = &mut app.screen {
                *scroll = scroll.saturating_sub(1);
            }
        }
        KeyCode::Char('t') => {
            if let Screen::ObservationDetail { obs, .. } = &app.screen {
                let id = obs.id;
                let project = obs.project.clone();
                if let Ok(items) = app.server.call_timeline(id, None, &project) {
                    app.push_screen(Screen::Timeline {
                        center: id,
                        items,
                        selected: 0,
                    });
                }
            }
        }
        _ => {}
    }
}
