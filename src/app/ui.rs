use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::diff;
use crate::highlight;
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
        AppState::CommentList => render_comment_view(app, frame, chunks[1]),
        _ => render_mr_list(app, frame, chunks[1]),
    }

    let current_navigation_text = vec![
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
        Span::styled(" | ", Style::default().fg(Color::White)),
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
        frame.render_widget(Clear, frame.area());
        let popup_block = Block::default()
            .title("Y/N")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));

        let exit_text = Text::styled(
            "Would you like to output the buffer as json? (y/n)",
            Style::default().fg(Color::Red),
        );
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
                ListItem::new(Text::from(vec![title_line, meta_line, Line::raw("")]))
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

/// Render a single comment filling the entire center area, with code context
/// and syntax highlighting when available.
fn render_comment_view(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.flat_notes.is_empty() {
        let empty =
            Paragraph::new("No comments found.").block(Block::bordered().title(" Comments "));
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.comment_list_state.selected().unwrap_or(0);
    let total = app.flat_notes.len();
    let note = &app.flat_notes[selected];

    let block_title = format!(
        " Comment {}/{} for MR !{} ",
        selected + 1,
        total,
        app.merge_request_id
    );
    let outer_block = Block::bordered().title(block_title);
    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // Build code context lines (if this is a diff note).
    let code_lines: Vec<Line<'_>> =
        if let (Some(file_path), Some(target_line)) = (&note.file_path, note.new_line) {
            let context_lines = diff::extract_context(&app.parsed_diff, file_path, target_line, 5);
            if context_lines.is_empty() {
                Vec::new()
            } else {
                let mut lines = Vec::new();
                // File path header
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        format!("{file_path}:{target_line}"),
                        Style::default().fg(Color::Cyan).bold(),
                    ),
                ]));
                lines.push(Line::raw(""));
                // Syntax-highlighted diff lines
                lines.extend(highlight::highlight_diff_lines(file_path, &context_lines));
                lines.push(Line::raw(""));
                lines
            }
        } else {
            Vec::new()
        };

    let code_height = code_lines.len() as u16;

    // Split inner area: code block on top (if present), then comment below.
    let constraints = if code_height > 0 {
        vec![
            Constraint::Length(code_height),
            Constraint::Length(1), // separator
            Constraint::Min(1),    // comment body
        ]
    } else {
        vec![Constraint::Min(1)]
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    if code_height > 0 {
        let code_widget = Paragraph::new(code_lines);
        frame.render_widget(code_widget, sections[0]);

        // Separator line
        let separator = Paragraph::new(Line::from(
            "─".repeat(sections[1].width as usize).dark_gray(),
        ));
        frame.render_widget(separator, sections[1]);

        // Comment body section
        render_comment_body(note, frame, sections[2]);
    } else {
        render_comment_body(note, frame, sections[0]);
    }
}

/// Render the comment author, timestamp, and body text into the given area.
fn render_comment_body(note: &super::FlatNote, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Author + timestamp header
    lines.push(Line::from(vec![
        Span::styled(
            note.author_username.clone(),
            Style::default().fg(Color::Cyan).bold(),
        ),
        Span::raw("  "),
        Span::styled(
            note.created_at.clone(),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    lines.push(Line::raw(""));

    // Comment body
    for body_line in note.body.lines() {
        lines.push(Line::from(body_line.to_string()));
    }

    let comment_widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(comment_widget, area);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
