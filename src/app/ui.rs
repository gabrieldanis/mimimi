use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
};

use crate::diff;
use crate::highlight;
use crate::types::AppState;

use super::App;

/// Accent colour used throughout the UI.
const ACCENT: Color = Color::Cyan;
/// Muted foreground for secondary text.
const MUTED: Color = Color::DarkGray;
/// Slightly brighter muted for borders / subtle chrome.
const SURFACE: Color = Color::Rgb(60, 60, 70);

pub fn render(app: &mut App, frame: &mut Frame) {
    let outer = Layout::vertical([
        Constraint::Length(4), // title bar
        Constraint::Min(1),    // main content
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    render_title_bar(frame, outer[0]);

    match app.app_state {
        AppState::CommentList | AppState::ConfirmResolve => {
            render_comment_view(app, frame, outer[1])
        }
        _ => render_mr_list(app, frame, outer[1]),
    }

    render_status_bar(app, frame, outer[2]);

    if let AppState::Exiting = app.app_state {
        render_exit_popup(frame);
    }
    if let AppState::ConfirmResolve = app.app_state {
        render_resolve_popup(app, frame);
    }
}

// ── Title bar ───────────────────────────────────────────────────────────────

fn render_title_bar(frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_set(border::PLAIN)
        .border_style(Style::default().fg(SURFACE))
        .padding(Padding::new(0, 0, 1, 0));

    let title = Line::from(vec![
        Span::styled(
            " mimimi ",
            Style::default().fg(Color::Black).bg(ACCENT).bold(),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            "merge request reviewer",
            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
        ),
    ]);

    let widget = Paragraph::new(title)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(widget, area);
}

// ── Status bar ──────────────────────────────────────────────────────────────

fn render_status_bar(app: &mut App, frame: &mut Frame, area: Rect) {
    let (left_spans, right_spans) = match app.app_state {
        AppState::MergeRequestList => (
            vec![
                Span::styled(" Merge Requests", Style::default().fg(ACCENT)),
                Span::styled(
                    format!("  {} items", app.merge_requests.len()),
                    Style::default().fg(MUTED),
                ),
            ],
            vec![
                key_hint("q", "quit"),
                Span::raw("  "),
                key_hint("enter", "select"),
                Span::raw("  "),
                key_hint("j/k", "navigate"),
            ],
        ),
        AppState::CommentList => {
            let selected = app.comment_list_state.selected().unwrap_or(0) + 1;
            let total = app.flat_threads.len();
            let sel_count = app.selected_threads.len();

            // Check for "Sent!" indicator (show for 2 seconds after HTTP send).
            let sent_active = app
                .sent_indicator
                .is_some_and(|t| t.elapsed().as_secs() < 2);
            if app
                .sent_indicator
                .is_some_and(|t| t.elapsed().as_secs() >= 2)
            {
                app.sent_indicator = None;
            }

            // Check for status message (show for 5 seconds).
            let status_msg_active = app
                .status_message
                .as_ref()
                .is_some_and(|(_, t)| t.elapsed().as_secs() < 5);
            if app
                .status_message
                .as_ref()
                .is_some_and(|(_, t)| t.elapsed().as_secs() >= 5)
            {
                app.status_message = None;
            }

            let selection_info = if sel_count > 0 {
                Span::styled(
                    format!("  [{sel_count} selected]"),
                    Style::default().fg(Color::Yellow).bold(),
                )
            } else {
                Span::raw("")
            };

            let left = if let Some((msg, _)) = &app.status_message {
                if status_msg_active {
                    vec![Span::styled(
                        format!(" {msg}"),
                        Style::default().fg(Color::Yellow).bold(),
                    )]
                } else {
                    vec![
                        Span::styled(
                            format!(" MR !{}", app.merge_request_id),
                            Style::default().fg(ACCENT),
                        ),
                        Span::styled(format!("  {selected}/{total}"), Style::default().fg(MUTED)),
                        selection_info,
                    ]
                }
            } else if sent_active {
                vec![
                    Span::styled(
                        " Sent to OpenCode ",
                        Style::default().fg(Color::Green).bold(),
                    ),
                    Span::styled(format!("  {selected}/{total}"), Style::default().fg(MUTED)),
                ]
            } else {
                vec![
                    Span::styled(
                        format!(" MR !{}", app.merge_request_id),
                        Style::default().fg(ACCENT),
                    ),
                    Span::styled(format!("  {selected}/{total}"), Style::default().fg(MUTED)),
                    selection_info,
                ]
            };

            (
                left,
                vec![
                    key_hint("esc", "back"),
                    Span::raw("  "),
                    key_hint("space", "select"),
                    Span::raw("  "),
                    key_hint("a", "all"),
                    Span::raw("  "),
                    key_hint("r", "resolve"),
                    Span::raw("  "),
                    key_hint("enter", "send"),
                    Span::raw("  "),
                    key_hint("j/k", "navigate"),
                ],
            )
        }
        AppState::ConfirmResolve => (
            vec![Span::styled(
                " Confirm resolve",
                Style::default().fg(Color::Yellow).bold(),
            )],
            vec![
                key_hint("y/enter", "confirm"),
                Span::raw("  "),
                key_hint("n/esc", "cancel"),
            ],
        ),
        AppState::Exiting => (
            vec![Span::styled(" Exiting", Style::default().fg(Color::Red))],
            vec![
                key_hint("esc", "cancel"),
                Span::raw("  "),
                key_hint("enter", "confirm"),
            ],
        ),
    };

    let left = Paragraph::new(Line::from(left_spans));
    let right = Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right);

    // Render both on same area; left is left-aligned, right is right-aligned.
    frame.render_widget(left, area);
    frame.render_widget(right, area);
}

/// Render a key hint like `[q] quit` with accent styling.
fn key_hint<'a>(key: &'a str, desc: &'a str) -> Span<'a> {
    // We return a single span with embedded formatting; for simplicity use
    // a composed string. For true multi-style we'd need a Line, but the
    // status bar already uses Line::from(vec![...]).
    // Instead, return two spans via a helper — caller collects.
    // Actually let's just style it simply.
    Span::styled(
        format!("[{key}] {desc}"),
        Style::default().fg(Color::Rgb(140, 140, 150)),
    )
}

// ── MR list ─────────────────────────────────────────────────────────────────

fn render_mr_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let inner = area.inner(Margin::new(1, 0));

    let items: Vec<ListItem> = if app.merge_requests.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "No merge requests found.",
            Style::default().fg(MUTED),
        )]))]
    } else {
        app.merge_requests
            .iter()
            .map(|mr| {
                let title_line = Line::from(vec![
                    Span::styled(format!("!{}", mr.iid), Style::default().fg(ACCENT).bold()),
                    Span::raw("  "),
                    Span::styled(mr.title.as_str(), Style::default().bold()),
                ]);
                let meta_line = Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("[{}]", mr.state), Style::default().fg(MUTED)),
                    Span::raw("  "),
                    Span::styled(
                        mr.author.username.as_str(),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(" -> ", Style::default().fg(MUTED)),
                    Span::styled(mr.target_branch.as_str(), Style::default().fg(Color::White)),
                ]);
                ListItem::new(Text::from(vec![title_line, meta_line, Line::raw("")]))
            })
            .collect()
    };

    let list = List::new(items)
        .highlight_symbol("  > ")
        .highlight_style(Style::default().fg(ACCENT));

    frame.render_stateful_widget(list, inner, &mut app.list_state);
}

// ── Comment view ────────────────────────────────────────────────────────────

fn render_comment_view(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.flat_threads.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "No discussion threads found.",
            Style::default().fg(MUTED),
        ));
        frame.render_widget(empty, area.inner(Margin::new(2, 1)));
        return;
    }

    let selected = app.comment_list_state.selected().unwrap_or(0);
    let thread = &app.flat_threads[selected];
    let is_selected = app.selected_threads.contains(&selected);

    let content_area = area.inner(Margin::new(2, 1));

    // Build code context lines (if this is a diff thread).
    let code_lines: Vec<Line<'_>> =
        if let (Some(file_path), Some(target_line)) = (&thread.file_path, thread.new_line) {
            let context_lines = diff::extract_context(&app.parsed_diff, file_path, target_line, 5);
            if context_lines.is_empty() {
                Vec::new()
            } else {
                let mut lines = Vec::new();
                // File path pill
                lines.push(Line::from(vec![Span::styled(
                    format!(" {file_path}:{target_line} "),
                    Style::default().fg(ACCENT).bold(),
                )]));
                lines.push(Line::raw(""));
                lines.extend(highlight::highlight_diff_lines(file_path, &context_lines));
                lines.push(Line::raw(""));
                lines
            }
        } else {
            Vec::new()
        };

    let code_height = code_lines.len() as u16;

    let constraints = if code_height > 0 {
        vec![
            Constraint::Length(code_height),
            Constraint::Length(1), // separator
            Constraint::Min(1),    // thread body
        ]
    } else {
        vec![Constraint::Min(1)]
    };

    let sections = Layout::vertical(constraints).split(content_area);

    if code_height > 0 {
        let code_widget = Paragraph::new(code_lines);
        frame.render_widget(code_widget, sections[0]);

        // Thin separator
        let sep_width = sections[1].width as usize;
        let separator = Paragraph::new(Line::from(Span::styled(
            "─".repeat(sep_width),
            Style::default().fg(SURFACE),
        )));
        frame.render_widget(separator, sections[1]);

        render_thread_body(thread, is_selected, frame, sections[2]);
    } else {
        render_thread_body(thread, is_selected, frame, sections[0]);
    }
}

/// Render all notes in a thread stacked, with the selection checkbox on the first note.
fn render_thread_body(
    thread: &super::FlatThread,
    is_selected: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Thread status header (checkbox + resolved indicator)
    let checkbox = if is_selected { "[x] " } else { "[ ] " };
    let mut header_spans = vec![Span::styled(
        checkbox,
        Style::default()
            .fg(if is_selected { Color::Yellow } else { MUTED })
            .bold(),
    )];
    if thread.resolved {
        header_spans.push(Span::styled(
            "RESOLVED ",
            Style::default().fg(Color::Green).bold(),
        ));
    }
    if thread.notes.len() > 1 {
        header_spans.push(Span::styled(
            format!("{} replies", thread.notes.len() - 1),
            Style::default().fg(MUTED),
        ));
    }
    lines.push(Line::from(header_spans));
    lines.push(Line::raw(""));

    // Render each note in the thread
    for (i, note) in thread.notes.iter().enumerate() {
        if i > 0 {
            // Visual separator between replies
            lines.push(Line::from(Span::styled(
                "  ┄┄┄",
                Style::default().fg(SURFACE),
            )));
            lines.push(Line::raw(""));
        }

        // Author + timestamp
        let indent = if i > 0 { "  " } else { "" };
        lines.push(Line::from(vec![
            Span::raw(indent.to_string()),
            Span::styled(
                note.author_username.clone(),
                Style::default().fg(ACCENT).bold(),
            ),
            Span::raw("  "),
            Span::styled(note.created_at.clone(), Style::default().fg(MUTED)),
        ]));
        lines.push(Line::raw(""));

        // Comment body (indent replies)
        for body_line in note.body.lines() {
            lines.push(Line::from(Span::styled(
                format!("{indent}{body_line}"),
                Style::default().fg(Color::White),
            )));
        }
        lines.push(Line::raw(""));
    }

    let widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

// ── Exit popup ──────────────────────────────────────────────────────────────

fn render_exit_popup(frame: &mut Frame) {
    frame.render_widget(Clear, frame.area());

    let area = centered_rect(50, 20, frame.area());

    let block = Block::new()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(ACCENT))
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::default().bg(Color::Rgb(25, 25, 35)));

    let text = Paragraph::new(vec![
        Line::from(Span::styled(
            "Output buffer as JSON?",
            Style::default().fg(Color::White).bold(),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(ACCENT).bold()),
            Span::raw(" yes   "),
            Span::styled("[n]", Style::default().fg(ACCENT).bold()),
            Span::raw(" no"),
        ]),
    ])
    .block(block)
    .alignment(Alignment::Center);

    frame.render_widget(text, area);
}

// ── Resolve confirmation popup ─────────────────────────────────────────────

fn render_resolve_popup(app: &App, frame: &mut Frame) {
    let indices = app.resolvable_selected_indices();
    let thread_nums: String = indices
        .iter()
        .map(|i| format!("#{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let area = centered_rect(60, 30, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(Color::Yellow))
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::default().bg(Color::Rgb(25, 25, 35)))
        .title(Span::styled(
            " Resolve Threads ",
            Style::default().fg(Color::Yellow).bold(),
        ));

    let text = Paragraph::new(vec![
        Line::from(Span::styled(
            "Mark the following threads as resolved?",
            Style::default().fg(Color::White).bold(),
        )),
        Line::raw(""),
        Line::from(Span::styled(thread_nums, Style::default().fg(ACCENT))),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[y/enter]", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" confirm   "),
            Span::styled("[n/esc]", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" cancel"),
        ]),
    ])
    .block(block)
    .wrap(Wrap { trim: false });

    frame.render_widget(text, area);
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
