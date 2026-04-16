mod keyboard_events;
mod ui;

use crate::diff::{self, ParsedDiff};
use crate::gitlab::{run_glab, run_glab_raw};
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
    /// Flattened non-system notes for indexed access in comment view.
    flat_notes: Vec<FlatNote>,
    /// Parsed diff data for the current MR.
    parsed_diff: ParsedDiff,
    list_state: ListState,
    comment_list_state: ListState,

    exit: bool,
}

/// A flattened note with pre-extracted position info for rendering.
#[derive(Debug)]
pub struct FlatNote {
    pub author_username: String,
    pub created_at: String,
    pub body: String,
    pub file_path: Option<String>,
    pub new_line: Option<usize>,
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
        let mr_str = selected_mr.to_string();
        self.merge_request_id = mr_str.clone();

        self.merge_request_comments = run_glab::<MergeRequestWithDiscussions>(&[
            "-R",
            "gitlab.com/glab-env/glab",
            "mr",
            "view",
            &mr_str,
            "--comments",
        ])
        .expect("Failed to fetch merge request comments");

        // Fetch raw diff for code context display.
        let raw_diff = run_glab_raw(&["mr", "diff", &mr_str, "-R", "gitlab.com/glab-env/glab"])
            .unwrap_or_default();
        self.parsed_diff = diff::parse_unified_diff(&raw_diff);

        // Flatten non-system notes for indexed access.
        self.flat_notes = self
            .merge_request_comments
            .discussions
            .iter()
            .flat_map(|d| d.notes.iter())
            .filter(|n| !n.system)
            .map(|note| {
                let (file_path, new_line) = match &note.position {
                    Some(pos) => (pos.new_path.clone(), pos.new_line),
                    None => (None, None),
                };
                FlatNote {
                    author_username: note.author.username.clone(),
                    created_at: note.created_at.clone(),
                    body: note.body.clone(),
                    file_path,
                    new_line,
                }
            })
            .collect();

        self.comment_list_state = if !self.flat_notes.is_empty() {
            ListState::default().with_selected(Some(0))
        } else {
            ListState::default()
        };

        self.app_state = AppState::CommentList;
    }
}
