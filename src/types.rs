//! Typed representations of GitLab API objects returned by `glab`.

#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Default, PartialEq)]
pub enum AppState {
    #[default]
    MergeRequestList,
    CommentList,
    Exiting,
}

/// A GitLab user as returned by the glab JSON output.
#[derive(Debug, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub state: String,
    pub web_url: String,
}

/// A GitLab merge request as returned by `glab mr list -F json`.
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
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub upvotes: u32,
    pub downvotes: u32,
    pub web_url: Option<String>,
}

/// A single note (comment) within a discussion thread, as returned by
/// `glab mr view <id> --comments -F json`.
#[derive(Debug, Deserialize)]
pub struct Note {
    pub id: u64,
    pub body: String,
    pub author: User,
    /// `true` for automated system events (e.g. "requested review from @x").
    pub system: bool,
    pub created_at: String,
    pub updated_at: String,
    pub resolvable: bool,
    pub resolved: bool,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<User>,
    pub internal: bool,
    pub confidential: bool,
    pub noteable_type: String,
    pub noteable_iid: Option<u64>,
}

/// A discussion thread, containing one or more [`Note`]s.
#[derive(Debug, Deserialize)]
pub struct Discussion {
    pub id: String,
    pub individual_note: bool,
    pub notes: Vec<Note>,
}

/// The response from `glab mr view <id> --comments -F json`: a full MR object
/// with its discussion threads included under the `Discussions` field.
#[derive(Debug, Deserialize)]
pub struct MergeRequestWithDiscussions {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub source_branch: String,
    pub target_branch: String,
    pub author: User,
    pub assignee: Option<User>,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub upvotes: u32,
    pub downvotes: u32,
    pub web_url: Option<String>,
    /// All discussion threads on this MR, each containing one or more notes.
    #[serde(rename = "Discussions")]
    pub discussions: Vec<Discussion>,
}
