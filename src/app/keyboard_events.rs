use super::App;
use crate::types::AppState;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.app_state {
            AppState::CommentList => match key_event.code {
                KeyCode::Esc => self.app_state = AppState::MergeRequestList,
                KeyCode::Up | KeyCode::Char('k') => self.comment_list_state.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => self.comment_list_state.select_next(),
                KeyCode::Enter => self.pending_send = true,
                _ => {}
            },
            _ => match key_event.code {
                KeyCode::Char('q') => self.exit = true,
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
}
