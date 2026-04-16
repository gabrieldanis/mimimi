mod keyboard_events;
mod ui;

use std::io;
use std::time::Instant;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::{DefaultTerminal, Frame, widgets::ListState};

use crate::diff::{self, ParsedDiff};
use crate::gitlab::{run_glab, run_glab_raw};
use crate::opencode;
use crate::types::AppState;
pub(crate) use crate::types::{MergeRequest, MergeRequestWithDiscussions};

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
    /// Set when a prompt was successfully sent via HTTP; UI shows a brief indicator.
    pub sent_indicator: Option<Instant>,
    /// Set by keyboard handler to request a send on the next main-loop tick.
    pending_send: bool,

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
    /// Runs the application's main loop until the user quits.
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

            if self.pending_send {
                self.pending_send = false;
                self.send_to_opencode(terminal);
            }
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

    /// Build a structured prompt string from the currently selected comment.
    fn build_prompt_for_current_comment(&self) -> Option<String> {
        let selected = self.comment_list_state.selected()?;
        let note = self.flat_notes.get(selected)?;

        let mr = &self.merge_request_comments;
        let description = mr
            .description
            .as_deref()
            .unwrap_or("No description provided.");

        let code_section = if let (Some(file_path), Some(target_line)) =
            (&note.file_path, note.new_line)
        {
            let context_lines = diff::extract_context(&self.parsed_diff, file_path, target_line, 5);
            if context_lines.is_empty() {
                format!(
                    "The following review comment was left on `{file_path}` at line {target_line}:"
                )
            } else {
                let diff_text: String = context_lines
                    .iter()
                    .map(|dl| {
                        let marker = match dl.kind {
                            diff::DiffLineKind::Add => "+",
                            diff::DiffLineKind::Remove => "-",
                            diff::DiffLineKind::Context => " ",
                        };
                        format!("{marker}{}\n", dl.content)
                    })
                    .collect();
                format!(
                    "The following review comment was left on `{file_path}` at line {target_line}:\n\n\
                     Code context:\n```\n{diff_text}```"
                )
            }
        } else {
            "The following general review comment was left on this merge request:".to_string()
        };

        let prompt = format!(
            "I'm reviewing merge request \"!{iid} {title}\".\n\n\
             MR Description:\n{description}\n\n\
             {code_section}\n\n\
             Comment:\n{body}\n\n\
             Think about this review comment carefully. Explain the pros and cons of the \
             suggested change, and propose a concrete implementation plan. Do not make any \
             changes -- only provide the plan.",
            iid = mr.iid,
            title = mr.title,
            description = description,
            code_section = code_section,
            body = note.body,
        );

        Some(prompt)
    }

    /// Send the current comment's prompt to OpenCode.
    ///
    /// Tries the HTTP API to a running instance first. If none is found,
    /// suspends the TUI and launches `opencode run` interactively.
    fn send_to_opencode(&mut self, terminal: &mut DefaultTerminal) {
        let Some(prompt) = self.build_prompt_for_current_comment() else {
            return;
        };

        // Try HTTP first.
        if let Some(port) = opencode::discover_opencode_port() {
            match opencode::send_prompt_http(port, &prompt) {
                Ok(()) => {
                    self.sent_indicator = Some(Instant::now());
                }
                Err(_e) => {
                    // HTTP send failed; fall through to interactive.
                    self.launch_opencode_interactive(terminal, &prompt);
                }
            }
            return;
        }

        // No running instance found — launch interactively.
        self.launch_opencode_interactive(terminal, &prompt);
    }

    /// Suspend the TUI, run `opencode run` with the given prompt, then restore.
    fn launch_opencode_interactive(&mut self, terminal: &mut DefaultTerminal, prompt: &str) {
        // Leave the TUI so opencode can take over.
        let _ = terminal.clear();
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);

        let _ = opencode::run_opencode_interactive(prompt);

        // Restore the TUI.
        let _ = enable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), EnterAlternateScreen);
        let _ = terminal.clear();
    }
}
