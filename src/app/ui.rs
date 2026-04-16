use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::types::AppState;

use super::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    // Create the layout sections.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    // TITLE
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let title = Paragraph::new(Text::styled(
        "Send Merge Requests to Opencode",
        Style::default().fg(Color::Magenta),
    ))
    .block(title_block);

    frame.render_widget(title, chunks[0]);

    match app.app_state {
        AppState::CommentList => render_comment_list(app, frame, chunks[1]),
        _ => render_mr_list(app, frame, chunks[1]),
    }

    let current_navigation_text = vec![
        // The first half of the text
        match app.app_state {
            AppState::MergeRequestList => {
                Span::styled("Merge Requests", Style::default().fg(Color::Green))
            }
            AppState::CommentList => {
                Span::styled("Threads and Comments", Style::default().fg(Color::Yellow))
            }
            AppState::Exiting => Span::styled("Exiting", Style::default().fg(Color::LightRed)),
        }
        .to_owned(),
        // A white divider bar to separate the two sections
        Span::styled(" | ", Style::default().fg(Color::White)),
        // The final section of the text, with hints on what the user is editing
        {
            if let AppState::CommentList = app.app_state {
                Span::styled(
                    format!("MR {}", app.merge_request_id),
                    Style::default().fg(Color::Green),
                )
            } else {
                Span::styled(
                    "No Merge Request selected",
                    Style::default().fg(Color::DarkGray),
                )
            }
        },
    ];

    let mode_footer = Paragraph::new(Line::from(current_navigation_text))
        .block(Block::default().borders(Borders::ALL));

    let current_keys_hint = {
        match app.app_state {
            AppState::MergeRequestList => Span::styled(
                "(q) to quit / (enter) to select",
                Style::default().fg(Color::Yellow),
            ),
            AppState::CommentList => Span::styled(
                "(ESC) to go back / (j/k) to navigate",
                Style::default().fg(Color::Yellow),
            ),
            AppState::Exiting => Span::styled(
                "(ESC) to abort / (enter) to exit",
                Style::default().fg(Color::Yellow),
            ),
        }
    };

    let key_notes_footer =
        Paragraph::new(Line::from(current_keys_hint)).block(Block::default().borders(Borders::ALL));

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    frame.render_widget(mode_footer, footer_chunks[0]);
    frame.render_widget(key_notes_footer, footer_chunks[1]);

    if let AppState::Exiting = app.app_state {
        frame.render_widget(Clear, frame.area()); //this clears the entire screen and anything already drawn
        let popup_block = Block::default()
            .title("Y/N")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));

        let exit_text = Text::styled(
            "Would you like to output the buffer as json? (y/n)",
            Style::default().fg(Color::Red),
        );
        // the `trim: false` will stop the text from being cut off when over the edge of the block
        let exit_paragraph = Paragraph::new(exit_text)
            .block(popup_block)
            .wrap(Wrap { trim: false });

        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(exit_paragraph, area);
    }
}

fn render_mr_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = if app.merge_requests.is_empty() {
        vec![ListItem::new(Text::from("No merge requests found."))]
    } else {
        app.merge_requests
            .iter()
            .map(|mr| {
                let title_line = Line::from(vec![
                    format!("!{}  ", mr.iid).into(),
                    mr.title.as_str().bold(),
                ]);
                let meta_line = Line::from(vec![
                    "   ".into(),
                    format!("[{}]", mr.state).dark_gray(),
                    "  ".into(),
                    mr.author.username.as_str().into(),
                    " → ".dark_gray(),
                    mr.target_branch.as_str().into(),
                ]);
                ListItem::new(Text::from(vec![
                    title_line,
                    meta_line,
                    Line::raw(""), // blank separator between cards
                ]))
            })
            .collect()
    };

    let block = Block::bordered();

    let list = List::new(items)
        .block(block)
        .highlight_symbol(Line::from("▶ ").cyan())
        .highlight_style(Style::default());

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_comment_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = {
        let notes: Vec<_> = app
            .merge_request_comments
            .discussions
            .iter()
            .flat_map(|d| d.notes.iter())
            .filter(|n| !n.system)
            .collect();

        if notes.is_empty() {
            vec![ListItem::new(Text::from("No comments found."))]
        } else {
            notes
                .iter()
                .map(|note| {
                    let header_line = Line::from(vec![
                        note.author.username.as_str().bold().fg(Color::Cyan),
                        "  ".into(),
                        note.created_at.as_str().dark_gray(),
                    ]);
                    // Preserve multi-line comment bodies.
                    let body_lines: Vec<Line> = note
                        .body
                        .lines()
                        .map(|l| Line::from(l.to_owned()))
                        .collect();
                    let mut all_lines = vec![header_line];
                    all_lines.extend(body_lines);
                    all_lines.push(Line::raw("")); // blank separator
                    ListItem::new(Text::from(all_lines))
                })
                .collect()
        }
    };

    let block = Block::bordered().title(format!(" Comments for MR !{} ", app.merge_request_id));

    let list = List::new(items)
        .block(block)
        .highlight_symbol(Line::from("▶ ").cyan())
        .highlight_style(Style::default());

    frame.render_stateful_widget(list, area, &mut app.comment_list_state);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
