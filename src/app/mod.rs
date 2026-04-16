mod keyboard_events;
mod ui;

use std::collections::HashSet;
use std::io;
use std::time::Instant;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{DefaultTerminal, Frame, widgets::ListState};

use crate::diff::{self, ParsedDiff};
use crate::gitlab::GitLabClient;
use crate::opencode;
use crate::types::AppState;
pub(crate) use crate::types::{MergeRequest, MergeRequestWithDiscussions};

#[derive(Debug)]
pub struct App {
    gitlab: GitLabClient,
    merge_requests: Vec<MergeRequest>,
    app_state: AppState,
    merge_request_id: String,
    merge_request_comments: MergeRequestWithDiscussions,
    /// Discussion threads with their notes, for indexed access in comment view.
    flat_threads: Vec<FlatThread>,
    /// Parsed diff data for the current MR.
    parsed_diff: ParsedDiff,
    list_state: ListState,
    comment_list_state: ListState,
    /// Set when a prompt was successfully sent via HTTP; UI shows a brief indicator.
    pub sent_indicator: Option<Instant>,
    /// Temporary status message shown in the status bar (with timestamp for auto-clear).
    pub status_message: Option<(String, Instant)>,
    /// Set by keyboard handler to request a send on the next main-loop tick.
    pending_send: bool,
    /// Indices of threads selected for batch send / resolve.
    pub selected_threads: HashSet<usize>,
    /// Set by keyboard handler to request resolving selected threads on next tick.
    pending_resolve: bool,

    exit: bool,
}

impl App {
    /// Create a new `App` with the given [`GitLabClient`].
    pub fn new(gitlab: GitLabClient) -> Self {
        Self {
            gitlab,
            merge_requests: Vec::new(),
            app_state: AppState::default(),
            merge_request_id: String::new(),
            merge_request_comments: MergeRequestWithDiscussions::default(),
            flat_threads: Vec::new(),
            parsed_diff: ParsedDiff::default(),
            list_state: ListState::default(),
            comment_list_state: ListState::default(),
            sent_indicator: None,
            status_message: None,
            pending_send: false,
            selected_threads: HashSet::new(),
            pending_resolve: false,
            exit: false,
        }
    }
}

/// A single note within a thread, with author and body info for rendering.
#[derive(Debug)]
pub struct ThreadNote {
    pub author_username: String,
    pub created_at: String,
    pub body: String,
}

/// A discussion thread with all its non-system notes and position info from the root note.
#[derive(Debug)]
pub struct FlatThread {
    pub discussion_id: String,
    /// Diff position from the root note (first note in thread).
    pub file_path: Option<String>,
    pub new_line: Option<usize>,
    /// All non-system notes in the thread, in order.
    pub notes: Vec<ThreadNote>,
    pub resolvable: bool,
    pub resolved: bool,
}

impl App {
    /// Runs the application's main loop until the user quits.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.merge_requests = self.gitlab.list_merge_requests().unwrap_or_default();
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

            if self.pending_resolve {
                self.pending_resolve = false;
                self.resolve_selected_comments();
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

        self.merge_request_comments = self
            .gitlab
            .get_merge_request_with_discussions(selected_mr)
            .expect("Failed to fetch merge request comments");

        // Fetch raw diff for code context display.
        let raw_diff = self
            .gitlab
            .get_merge_request_diff(selected_mr)
            .unwrap_or_default();
        self.parsed_diff = diff::parse_unified_diff(&raw_diff);

        // Build thread list from discussions, filtering out system notes.
        self.flat_threads = self
            .merge_request_comments
            .discussions
            .iter()
            .filter_map(|d| {
                let notes: Vec<ThreadNote> = d
                    .notes
                    .iter()
                    .filter(|n| !n.system)
                    .map(|n| ThreadNote {
                        author_username: n.author.username.clone(),
                        created_at: n.created_at.clone(),
                        body: n.body.clone(),
                    })
                    .collect();

                if notes.is_empty() {
                    return None;
                }

                // Position info comes from the first note with a position.
                let root_pos = d.notes.iter().find_map(|n| n.position.as_ref());
                let (file_path, new_line) = match root_pos {
                    Some(pos) => (pos.new_path.clone(), pos.new_line),
                    None => (None, None),
                };

                // Resolvable/resolved from the discussion's first resolvable note.
                let resolvable_note = d.notes.iter().find(|n| n.resolvable);
                let resolvable = resolvable_note.is_some();
                let resolved = resolvable_note.is_some_and(|n| n.resolved);

                Some(FlatThread {
                    discussion_id: d.id.clone(),
                    file_path,
                    new_line,
                    notes,
                    resolvable,
                    resolved,
                })
            })
            .collect();

        self.comment_list_state = if !self.flat_threads.is_empty() {
            ListState::default().with_selected(Some(0))
        } else {
            ListState::default()
        };
        self.selected_threads.clear();

        self.app_state = AppState::CommentList;
    }

    /// Build a structured prompt string for a single thread by index.
    fn build_prompt_for_thread(&self, idx: usize) -> Option<String> {
        let thread = self.flat_threads.get(idx)?;

        let code_section = if let (Some(file_path), Some(target_line)) =
            (&thread.file_path, thread.new_line)
        {
            let context_lines = diff::extract_context(&self.parsed_diff, file_path, target_line, 5);
            if context_lines.is_empty() {
                format!(
                    "The following review thread was left on `{file_path}` at line {target_line}:"
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
                    "The following review thread was left on `{file_path}` at line {target_line}:\n\n\
                     Code context:\n```\n{diff_text}```"
                )
            }
        } else {
            "The following general review thread was left on this merge request:".to_string()
        };

        let comments: String = thread
            .notes
            .iter()
            .map(|n| format!("{} ({}):\n{}", n.author_username, n.created_at, n.body))
            .collect::<Vec<_>>()
            .join("\n\n");

        Some(format!("{code_section}\n\n{comments}"))
    }

    /// Build a prompt from the selected threads (or current thread if none selected).
    fn build_prompt_for_selection(&self) -> Option<String> {
        let indices: Vec<usize> = if self.selected_threads.is_empty() {
            // Fall back to the currently focused thread.
            vec![self.comment_list_state.selected()?]
        } else {
            let mut v: Vec<usize> = self.selected_threads.iter().copied().collect();
            v.sort_unstable();
            v
        };

        let mr = &self.merge_request_comments;
        let description = mr
            .description
            .as_deref()
            .unwrap_or("No description provided.");

        let mut sections: Vec<String> = Vec::new();
        for (i, &idx) in indices.iter().enumerate() {
            if let Some(section) = self.build_prompt_for_thread(idx) {
                sections.push(format!("### Thread {}\n\n{section}", i + 1));
            }
        }

        if sections.is_empty() {
            return None;
        }

        let prompt = format!(
            "I'm reviewing merge request \"!{iid} {title}\".\n\n\
             MR Description:\n{description}\n\n\
             {threads}\n\n\
             Think about {noun} carefully. Explain the pros and cons of the \
             suggested {change_noun}, and propose a concrete implementation plan. Do not make any \
             changes -- only provide the plan.",
            iid = mr.iid,
            title = mr.title,
            description = description,
            threads = sections.join("\n\n---\n\n"),
            noun = if sections.len() == 1 {
                "this review thread"
            } else {
                "these review threads"
            },
            change_noun = if sections.len() == 1 {
                "change"
            } else {
                "changes"
            },
        );

        Some(prompt)
    }

    /// Send the current comment's prompt to OpenCode.
    ///
    /// 1. Try sending via HTTP to an already-running instance.
    /// 2. If no instance is found, try launching opencode in a new terminal
    ///    window, wait for it to become ready, and send via HTTP.
    /// 3. If that also fails, show a status message asking the user to start
    ///    opencode manually.
    fn send_to_opencode(&mut self, _terminal: &mut DefaultTerminal) {
        let Some(prompt) = self.build_prompt_for_selection() else {
            return;
        };

        // 1. Try sending to an already-running instance.
        if let Some(port) = opencode::discover_opencode_port()
            && opencode::send_prompt_http(port, &prompt).is_ok()
        {
            self.sent_indicator = Some(Instant::now());
            return;
        }

        // 2. Try launching opencode in a new window and sending.
        match opencode::launch_and_send(&prompt) {
            Ok(()) => {
                self.sent_indicator = Some(Instant::now());
            }
            Err(_e) => {
                // 3. Give up — tell the user to start opencode manually.
                self.status_message = Some((
                    "No opencode instance found. Run 'opencode --port' in this directory."
                        .to_string(),
                    Instant::now(),
                ));
            }
        }
    }

    /// Return sorted indices of selected resolvable (and not yet resolved) threads.
    pub fn resolvable_selected_indices(&self) -> Vec<usize> {
        let indices: Box<dyn Iterator<Item = usize>> = if self.selected_threads.is_empty() {
            if let Some(idx) = self.comment_list_state.selected() {
                Box::new(std::iter::once(idx))
            } else {
                return Vec::new();
            }
        } else {
            Box::new(self.selected_threads.iter().copied())
        };

        let mut result: Vec<usize> = indices
            .filter(|&i| {
                self.flat_threads
                    .get(i)
                    .is_some_and(|t| t.resolvable && !t.resolved)
            })
            .collect();
        result.sort_unstable();
        result
    }

    /// Resolve all selected threads via the GitLab API.
    fn resolve_selected_comments(&mut self) {
        let indices = self.resolvable_selected_indices();
        if indices.is_empty() {
            self.status_message = Some((
                "No resolvable threads in selection.".to_string(),
                Instant::now(),
            ));
            return;
        }

        let mut resolved_count = 0u32;
        let mut errors = Vec::new();
        let mr_iid = self.merge_request_comments.iid;

        for &idx in &indices {
            let discussion_id = self.flat_threads[idx].discussion_id.clone();
            match self.gitlab.resolve_discussion(mr_iid, &discussion_id) {
                Ok(()) => {
                    resolved_count += 1;
                    self.flat_threads[idx].resolved = true;
                }
                Err(e) => errors.push(e),
            }
        }

        let msg = if errors.is_empty() {
            format!("Resolved {resolved_count} thread(s).")
        } else {
            format!(
                "Resolved {resolved_count}, failed {}: {}",
                errors.len(),
                errors[0]
            )
        };
        self.status_message = Some((msg, Instant::now()));
        self.selected_threads.clear();
    }
}
