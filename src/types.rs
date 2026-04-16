//! Typed representations of GitLab API objects.

#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Default, PartialEq)]
pub enum AppState {
    #[default]
    MergeRequestList,
    CommentList,
    /// Confirmation popup for resolving selected comments.
    ConfirmResolve,
    Exiting,
}

/// A GitLab user as returned by the GitLab REST API.
#[derive(Debug, Deserialize, Default)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub state: String,
    #[serde(default)]
    pub web_url: String,
}

/// A GitLab merge request as returned by `GET /projects/:id/merge_requests`.
#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub source_branch: String,
    pub target_branch: String,
    pub author: User,
    pub assignee: Option<User>,
    #[serde(default)]
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub upvotes: u32,
    #[serde(default)]
    pub downvotes: u32,
    pub web_url: Option<String>,
}

/// Position information for a diff note, indicating which file and line(s)
/// the comment refers to.
#[derive(Debug, Deserialize, Clone)]
pub struct NotePosition {
    pub base_sha: Option<String>,
    pub start_sha: Option<String>,
    pub head_sha: Option<String>,
    pub position_type: Option<String>,
    pub new_path: Option<String>,
    pub old_path: Option<String>,
    pub new_line: Option<usize>,
    pub old_line: Option<usize>,
}

/// A single note (comment) within a discussion thread.
#[derive(Debug, Deserialize)]
pub struct Note {
    pub id: u64,
    #[serde(rename = "type", default)]
    pub note_type: Option<String>,
    pub body: String,
    pub author: User,
    /// `true` for automated system events (e.g. "requested review from @x").
    #[serde(default)]
    pub system: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub resolvable: bool,
    #[serde(default)]
    pub resolved: bool,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<User>,
    pub position: Option<NotePosition>,
    #[serde(default)]
    pub internal: bool,
    #[serde(default)]
    pub confidential: bool,
    #[serde(default)]
    pub noteable_type: String,
    pub noteable_iid: Option<u64>,
}

/// A discussion thread, containing one or more [`Note`]s.
#[derive(Debug, Deserialize)]
pub struct Discussion {
    pub id: String,
    #[serde(default)]
    pub individual_note: bool,
    pub notes: Vec<Note>,
}

/// A full MR object combined with its discussion threads.
///
/// The MR fields come from `GET /projects/:id/merge_requests/:iid` and the
/// discussions come from a separate discussions endpoint; they are merged by
/// [`crate::gitlab::GitLabClient::get_merge_request_with_discussions`].
#[derive(Debug, Deserialize, Default)]
pub struct MergeRequestWithDiscussions {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub iid: u64,
    #[serde(default)]
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub source_branch: String,
    #[serde(default)]
    pub target_branch: String,
    #[serde(default)]
    pub author: User,
    pub assignee: Option<User>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub upvotes: u32,
    #[serde(default)]
    pub downvotes: u32,
    pub web_url: Option<String>,
    /// Discussion threads — populated separately from the discussions API.
    #[serde(default)]
    pub discussions: Vec<Discussion>,
}
