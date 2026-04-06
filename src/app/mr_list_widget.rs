use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, List, ListItem, Padding},
};

use super::App;

pub fn render(app: &mut App, area: Rect, frame: &mut Frame) {
    // Center a 60-column box horizontally and vertically.
    const BOX_WIDTH: u16 = 80;
    let [_, center_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Fill(3),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, center, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(BOX_WIDTH),
        Constraint::Fill(1),
    ])
    .areas(center_v);

    let title = Line::from(" Merge Requests ".bold());
    let instructions = Line::from(vec![
        " ↑/k ".into(),
        "prev".dark_gray(),
        "  ↓/j ".into(),
        "next".dark_gray(),
        "  ↵ ".into(),
        "select".dark_gray(),
        "  q ".into(),
        "quit ".dark_gray(),
    ]);
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(instructions.centered())
        .border_set(border::THICK)
        .padding(Padding::new(2, 2, 1, 1));

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

    let list = List::new(items)
        .block(block)
        .highlight_symbol(Line::from("▶ ").cyan())
        .highlight_style(Style::default());

    ratatui::widgets::StatefulWidget::render(list, center, frame.buffer_mut(), &mut app.list_state);
}
