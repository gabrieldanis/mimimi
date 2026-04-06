mod comments;
mod keyboard_events;
mod mr_list_widget;

use crate::gitlab::run_glab;
use crate::types::{AppState, MergeRequest};
use std::{io, string};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    widgets::{ListState, Widget},
};

#[derive(Debug, Default)]
pub struct App {
    merge_requests: Vec<MergeRequest>,
    app_state: AppState,
    current_merge_request: String,
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
        self.render(area, frame);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn render(&mut self, area: Rect, frame: &mut Frame) {
        // if self.app_state == AppState::MergeRequestList {
        //     mr_list_widget::render(self, area, frame);
        // }
        comments::render(self, area, frame);
    }
}
