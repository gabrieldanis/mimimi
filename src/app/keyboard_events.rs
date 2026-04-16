use super::App;
use crate::types::AppState;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.app_state {
            AppState::ConfirmResolve => match key_event.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.app_state = AppState::CommentList;
                    self.pending_resolve = true;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.app_state = AppState::CommentList;
                }
                _ => {}
            },
            AppState::CommentList => match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.app_state = AppState::MergeRequestList;
                }
                KeyCode::Up | KeyCode::Char('k') => self.select_prev_comment_wrapping(),
                KeyCode::Down | KeyCode::Char('j') => self.select_next_comment_wrapping(),
                KeyCode::Char(' ') => self.toggle_current_thread_selection(),
                KeyCode::Char('a') => self.toggle_select_all_threads(),
                KeyCode::Char('r') => {
                    if !self.resolvable_selected_indices().is_empty() {
                        self.app_state = AppState::ConfirmResolve;
                    }
                }
                KeyCode::Enter => self.pending_send = true,
                _ => {}
            },
            _ => match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') => self.exit = true,
                KeyCode::Up | KeyCode::Char('k') => self.list_state.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => self.list_state.select_next(),
                KeyCode::Enter => self.fetch_merge_request_comments(
                    self.merge_requests
                        .get(self.list_state.selected().unwrap_or(0))
                        .map(|mr| mr.iid)
                        .unwrap_or(0),
                ),
                _ => {}
            },
        }
    }

    fn select_next_comment_wrapping(&mut self) {
        let len = self.flat_threads.len();
        if len == 0 {
            return;
        }
        let current = self.comment_list_state.selected().unwrap_or(0);
        let next = if current + 1 >= len { 0 } else { current + 1 };
        self.comment_list_state.select(Some(next));
    }

    fn select_prev_comment_wrapping(&mut self) {
        let len = self.flat_threads.len();
        if len == 0 {
            return;
        }
        let current = self.comment_list_state.selected().unwrap_or(0);
        let prev = if current == 0 { len - 1 } else { current - 1 };
        self.comment_list_state.select(Some(prev));
    }

    fn toggle_current_thread_selection(&mut self) {
        if let Some(idx) = self.comment_list_state.selected()
            && !self.selected_threads.remove(&idx)
        {
            self.selected_threads.insert(idx);
        }
    }

    fn toggle_select_all_threads(&mut self) {
        let len = self.flat_threads.len();
        if self.selected_threads.len() == len {
            self.selected_threads.clear();
        } else {
            self.selected_threads = (0..len).collect();
        }
    }
}
