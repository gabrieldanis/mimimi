//! GitLab REST API client.
//!
//! Replaces the previous `glab` CLI wrapper with direct HTTP calls using `ureq`.
//! Detects the GitLab host and project path from the git remote, and manages
//! the personal access token via the OS keyring (with env-var fallback).

use std::io::{self, Write};
use std::process::Command;

use serde::Deserialize;

use crate::types::{Discussion, MergeRequest, MergeRequestWithDiscussions};

const KEYRING_SERVICE: &str = "mimimi";
const KEYRING_USER: &str = "gitlab-token";
const ENV_TOKEN: &str = "GITLAB_TOKEN";

/// Holds the resolved GitLab connection info needed for API calls.
#[derive(Debug, Clone)]
pub struct GitLabClient {
    /// Base API URL, e.g. `https://git.fronius.com/api/v4`.
    api_base: String,
    /// URL-encoded project path, e.g. `Danis.Gabriel%2Fmimimi`.
    project_id: String,
    /// Personal access token.
    token: String,
}

/// Errors that can occur when constructing a [`GitLabClient`].
#[derive(Debug)]
pub enum GitLabError {
    /// Could not determine the git remote URL.
    NoRemote(String),
    /// Could not parse the remote URL into host + project path.
    ParseError(String),
    /// No token available (neither env var nor keyring).
    NoToken(String),
}

impl std::fmt::Display for GitLabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitLabError::NoRemote(msg) => write!(f, "git remote error: {msg}"),
            GitLabError::ParseError(msg) => write!(f, "remote URL parse error: {msg}"),
            GitLabError::NoToken(msg) => write!(f, "GitLab token error: {msg}"),
        }
    }
}

impl std::error::Error for GitLabError {}

impl GitLabClient {
    /// Create a new client by auto-detecting the GitLab host/project from git
    /// and resolving a personal access token.
    pub fn from_git_remote() -> Result<Self, GitLabError> {
        let remote_url = detect_remote_url()?;
        let (host, project_path) = parse_remote_url(&remote_url)?;
        let api_base = format!("https://{host}/api/v4");
        let project_id = urlencoding_encode(&project_path);
        let token = resolve_token()?;

        Ok(Self {
            api_base,
            project_id,
            token,
        })
    }

    /// List open merge requests for the project.
    pub fn list_merge_requests(&self) -> Option<Vec<MergeRequest>> {
        let url = format!(
            "{}/projects/{}/merge_requests?state=opened&per_page=100",
            self.api_base, self.project_id
        );
        self.get_json(&url)
    }

    /// Fetch a single merge request by IID, together with its discussion
    /// threads (comments).
    pub fn get_merge_request_with_discussions(
        &self,
        mr_iid: u64,
    ) -> Option<MergeRequestWithDiscussions> {
        let mr_url = format!(
            "{}/projects/{}/merge_requests/{mr_iid}",
            self.api_base, self.project_id
        );
        let discussions_url = format!(
            "{}/projects/{}/merge_requests/{mr_iid}/discussions?per_page=100",
            self.api_base, self.project_id
        );

        let mr: MergeRequestWithDiscussions = self.get_json(&mr_url)?;
        let discussions: Vec<Discussion> = self.get_json(&discussions_url).unwrap_or_default();

        Some(MergeRequestWithDiscussions { discussions, ..mr })
    }

    /// Resolve a discussion thread on a merge request.
    ///
    /// Uses `PUT /projects/:id/merge_requests/:iid/discussions/:discussion_id`
    /// with `resolved=true`.
    pub fn resolve_discussion(&self, mr_iid: u64, discussion_id: &str) -> Result<(), String> {
        let url = format!(
            "{}/projects/{}/merge_requests/{mr_iid}/discussions/{discussion_id}?resolved=true",
            self.api_base, self.project_id
        );

        let response = ureq::put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send_empty();

        match response {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to resolve discussion {discussion_id}: {e}")),
        }
    }

    /// Fetch the raw unified diff of a merge request.
    ///
    /// Uses the merge request changes endpoint and reconstructs a unified diff
    /// string from the structured response so the existing diff parser keeps
    /// working.
    pub fn get_merge_request_diff(&self, mr_iid: u64) -> Option<String> {
        let url = format!(
            "{}/projects/{}/merge_requests/{mr_iid}/changes",
            self.api_base, self.project_id
        );

        #[derive(Deserialize)]
        struct Change {
            old_path: String,
            new_path: String,
            diff: String,
        }

        #[derive(Deserialize)]
        struct MrChanges {
            changes: Vec<Change>,
        }

        let mr_changes: MrChanges = self.get_json(&url)?;

        // Reconstruct unified diff text from the structured changes.
        let mut unified = String::new();
        for change in &mr_changes.changes {
            unified.push_str(&format!("--- a/{}\n", change.old_path));
            unified.push_str(&format!("+++ b/{}\n", change.new_path));
            unified.push_str(&change.diff);
            if !change.diff.ends_with('\n') {
                unified.push('\n');
            }
        }

        Some(unified)
    }

    /// Perform a GET request and deserialize the JSON body.
    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Option<T> {
        let response = ureq::get(url).header("PRIVATE-TOKEN", &self.token).call();

        match response {
            Ok(mut resp) => {
                let body = match resp.body_mut().read_to_string() {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("Error: failed to read GitLab API response body: {e}");
                        return None;
                    }
                };
                match serde_json::from_str::<T>(&body) {
                    Ok(value) => Some(value),
                    Err(e) => {
                        eprintln!("Error: failed to parse GitLab API response: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: GitLab API request failed: {e}");
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Git remote detection
// ---------------------------------------------------------------------------

/// Run `git remote get-url origin` and return the URL.
fn detect_remote_url() -> Result<String, GitLabError> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|e| GitLabError::NoRemote(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        return Err(GitLabError::NoRemote(
            "no 'origin' remote configured".into(),
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        return Err(GitLabError::NoRemote("origin remote URL is empty".into()));
    }
    Ok(url)
}

/// Parse a git remote URL (SSH or HTTPS) into `(host, project_path)`.
///
/// Supported formats:
/// - `git@host:group/project.git`
/// - `ssh://git@host/group/project.git`
/// - `https://host/group/project.git`
fn parse_remote_url(url: &str) -> Result<(String, String), GitLabError> {
    // SSH shorthand: git@host:path.git
    if let Some(rest) = url.strip_prefix("git@")
        && let Some((host, path)) = rest.split_once(':')
    {
        let project = path.trim_end_matches(".git").to_string();
        return Ok((host.to_string(), project));
    }

    // ssh://git@host/path.git or https://host/path.git
    if url.starts_with("ssh://") || url.starts_with("https://") || url.starts_with("http://") {
        // Strip scheme
        let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
        // Strip optional user@ prefix
        let without_user = if let Some((_user, rest)) = without_scheme.split_once('@') {
            rest
        } else {
            without_scheme
        };
        // Split host / path
        if let Some((host, path)) = without_user.split_once('/') {
            let project = path.trim_end_matches(".git").to_string();
            if !host.is_empty() && !project.is_empty() {
                return Ok((host.to_string(), project));
            }
        }
    }

    Err(GitLabError::ParseError(format!(
        "unrecognised remote URL format: {url}"
    )))
}

// ---------------------------------------------------------------------------
// Token management
// ---------------------------------------------------------------------------

/// Resolve a GitLab personal access token. Priority:
///
/// 1. `GITLAB_TOKEN` environment variable
/// 2. OS keyring (Windows Credential Manager)
/// 3. Interactive prompt (then stored in keyring)
fn resolve_token() -> Result<String, GitLabError> {
    // 1. Environment variable.
    if let Ok(token) = std::env::var(ENV_TOKEN)
        && !token.is_empty()
    {
        return Ok(token);
    }

    // 2. Keyring.
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        && let Ok(token) = entry.get_password()
        && !token.is_empty()
    {
        return Ok(token);
    }

    // 3. Interactive prompt.
    prompt_and_store_token()
}

/// Prompt the user for a token on stdin, store it in the keyring, and return it.
fn prompt_and_store_token() -> Result<String, GitLabError> {
    eprint!("Enter your GitLab personal access token: ");
    io::stderr().flush().ok();

    let mut token = String::new();
    io::stdin()
        .read_line(&mut token)
        .map_err(|e| GitLabError::NoToken(format!("failed to read token from stdin: {e}")))?;

    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(GitLabError::NoToken("empty token provided".into()));
    }

    // Persist in keyring for next time.
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        && let Err(e) = entry.set_password(&token)
    {
        eprintln!("Warning: could not save token to keyring: {e}");
    }

    Ok(token)
}

// ---------------------------------------------------------------------------
// Minimal percent-encoding for project paths
// ---------------------------------------------------------------------------

/// Percent-encode a project path for use in GitLab API URLs.
/// Only `/` needs to become `%2F`; other characters in GitLab paths are safe.
fn urlencoding_encode(input: &str) -> String {
    input.replace('/', "%2F")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_shorthand() {
        let (host, path) =
            parse_remote_url("git@git.fronius.com:Danis.Gabriel/mimimi.git").unwrap();
        assert_eq!(host, "git.fronius.com");
        assert_eq!(path, "Danis.Gabriel/mimimi");
    }

    #[test]
    fn test_parse_https() {
        let (host, path) =
            parse_remote_url("https://gitlab.com/group/subgroup/project.git").unwrap();
        assert_eq!(host, "gitlab.com");
        assert_eq!(path, "group/subgroup/project");
    }

    #[test]
    fn test_parse_ssh_scheme() {
        let (host, path) = parse_remote_url("ssh://git@gitlab.example.com/team/repo.git").unwrap();
        assert_eq!(host, "gitlab.example.com");
        assert_eq!(path, "team/repo");
    }

    #[test]
    fn test_parse_no_dot_git_suffix() {
        let (host, path) = parse_remote_url("git@host.com:a/b").unwrap();
        assert_eq!(host, "host.com");
        assert_eq!(path, "a/b");
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(
            urlencoding_encode("Danis.Gabriel/mimimi"),
            "Danis.Gabriel%2Fmimimi"
        );
        assert_eq!(
            urlencoding_encode("group/sub/project"),
            "group%2Fsub%2Fproject"
        );
    }
}
