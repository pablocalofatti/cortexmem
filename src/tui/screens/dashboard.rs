use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // title bar
            Constraint::Min(10),   // stats panel
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
            " dashboard ",
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

    // Stats panel
    let stats = app.server.call_stats(None);
    let stats_block = Block::default()
        .title(Span::styled(
            " Statistics ",
            Style::default().fg(theme::BLUE),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::SURFACE1))
        .style(Style::default().bg(theme::BASE));

    match stats {
        Ok(stats) => {
            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // total count
                    Constraint::Min(3),    // tier table
                    Constraint::Min(3),    // type table
                ])
                .margin(1)
                .split(chunks[1]);

            // Render the outer block
            f.render_widget(stats_block, chunks[1]);

            // Total count
            let total_line = Paragraph::new(Line::from(vec![
                Span::styled("Total observations: ", Style::default().fg(theme::SUBTEXT)),
                Span::styled(
                    stats.total.to_string(),
                    Style::default().fg(theme::GREEN),
                ),
            ]))
            .style(Style::default().bg(theme::BASE));
            f.render_widget(total_line, inner_chunks[0]);

            // By tier table
            let tier_rows: Vec<Row> = stats
                .by_tier
                .iter()
                .map(|(tier, count)| {
                    Row::new(vec![
                        Span::styled(tier.clone(), Style::default().fg(theme::LAVENDER)),
                        Span::styled(count.to_string(), Style::default().fg(theme::TEXT)),
                    ])
                })
                .collect();
            let tier_table = Table::new(
                tier_rows,
                [Constraint::Percentage(50), Constraint::Percentage(50)],
            )
            .header(
                Row::new(vec![
                    Span::styled("Tier", Style::default().fg(theme::YELLOW)),
                    Span::styled("Count", Style::default().fg(theme::YELLOW)),
                ])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title(Span::styled(
                        " By Tier ",
                        Style::default().fg(theme::BLUE),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::SURFACE1)),
            )
            .style(Style::default().bg(theme::BASE));
            f.render_widget(tier_table, inner_chunks[1]);

            // By type table
            let type_rows: Vec<Row> = stats
                .by_type
                .iter()
                .map(|(obs_type, count)| {
                    Row::new(vec![
                        Span::styled(obs_type.clone(), Style::default().fg(theme::LAVENDER)),
                        Span::styled(count.to_string(), Style::default().fg(theme::TEXT)),
                    ])
                })
                .collect();
            let type_table = Table::new(
                type_rows,
                [Constraint::Percentage(50), Constraint::Percentage(50)],
            )
            .header(
                Row::new(vec![
                    Span::styled("Type", Style::default().fg(theme::YELLOW)),
                    Span::styled("Count", Style::default().fg(theme::YELLOW)),
                ])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title(Span::styled(
                        " By Type ",
                        Style::default().fg(theme::BLUE),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::SURFACE1)),
            )
            .style(Style::default().bg(theme::BASE));
            f.render_widget(type_table, inner_chunks[2]);
        }
        Err(_) => {
            let error_msg = Paragraph::new(Span::styled(
                "Failed to load statistics",
                Style::default().fg(theme::RED),
            ))
            .block(stats_block);
            f.render_widget(error_msg, chunks[1]);
        }
    }

    // Keybindings help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" q ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Quit ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" s/", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled("/ ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Search ", Style::default().fg(theme::SUBTEXT)),
        Span::styled(" n ", Style::default().fg(theme::BASE).bg(theme::MAUVE)),
        Span::styled(" Sessions ", Style::default().fg(theme::SUBTEXT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme::SURFACE1)),
    )
    .style(Style::default().bg(theme::BASE));
    f.render_widget(help, chunks[2]);
}
