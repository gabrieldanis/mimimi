mod keyboard_events;
mod ui;

use crate::gitlab::run_glab;
use crate::types::AppState;
pub(crate) use crate::types::{MergeRequest, MergeRequestWithDiscussions};
use std::io;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{DefaultTerminal, Frame, widgets::ListState};

#[derive(Debug, Default)]
pub struct App {
    merge_requests: Vec<MergeRequest>,
    app_state: AppState,
    merge_request_id: String,
    merge_request_comments: MergeRequestWithDiscussions,
    list_state: ListState,
    comment_list_state: ListState,

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
        self.render(frame);
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

    fn render(&mut self, frame: &mut Frame) {
        ui::render(self, frame);
    }

    fn fetch_merge_request_comments(&mut self, selected_mr: u64) {
        self.merge_request_id = selected_mr.to_string();
        self.merge_request_comments = run_glab::<MergeRequestWithDiscussions>(&[
            "-R",
            "gitlab.com/glab-env/glab",
            "mr",
            "view",
            &selected_mr.to_string(),
            "--comments",
        ])
        .expect("Failed to fetch merge request comments");

        let note_count: usize = self
            .merge_request_comments
            .discussions
            .iter()
            .map(|d| d.notes.len())
            .sum();
        self.comment_list_state = if note_count > 0 {
            ListState::default().with_selected(Some(0))
        } else {
            ListState::default()
        };

        self.app_state = AppState::CommentList;
    }
}
