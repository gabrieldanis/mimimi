mod keyboard_events;

use crate::gitlab::run_glab;
use crate::types::{AppState, MergeRequest};
use std::io;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, List, ListItem, ListState, Padding, Widget},
};

#[derive(Debug, Default)]
pub struct App {
    merge_requests: Vec<MergeRequest>,
    app_state: AppState,
    list_state: ListState,
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.merge_requests =
            run_glab::<Vec<MergeRequest>>(&["mr", "list", "-R", "gitlab.com/glab-env/glab"])
                .unwrap_or_default();
        if !self.merge_requests.is_empty() {
            self.list_state = ListState::default().with_selected(Some(0));
        }
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        self.render(area, buf);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if self.app_state == AppState::MergeRequestList {
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

            let items: Vec<ListItem> = if self.merge_requests.is_empty() {
                vec![ListItem::new(Text::from("No merge requests found."))]
            } else {
                self.merge_requests
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

            ratatui::widgets::StatefulWidget::render(list, center, buf, &mut self.list_state);
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render(area, buf);
    }
}
