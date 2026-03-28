use std::{
    io,
    process::{Command, ExitStatus},
};

use serde::Deserialize;

fn main() {
    println!("Checking if glab is installed...");
    if is_glab_installed() {
        println!("glab is installed.");
    } else {
        println!("glab is not installed. Install it from https://gitlab.com/gitlab-org/cli");
    }
    println!("Looking for Merge Requests...");
    // TODO: handle failure of find_merge_requests
    find_merge_requests();
    print_merge_request_comments();
}

fn print_merge_request_comments() {
    let mut input = String::new();

    println!("To list the comments of a merge request enter the number on the left");

    // Read a line from standard input
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    // Convert the input string to a number (i32 in this example)
    let number: i32 = input
        .trim() // Remove whitespace/newline
        .parse() // Try to parse as i32
        .expect("Please enter a valid number");

    println!("You entered: {}", number);

    if let Some(mr) = run_glab::<MergeRequestWithDiscussions>(&[
        "-R",
        "gitlab.com/glab-env/glab",
        "mr",
        "view",
        &number.to_string(),
        "--comments",
    ]) {
        for discussion in &mr.discussions {
            for note in &discussion.notes {
                println!("{}: {}", note.author.username, note.body);
            }
        }
    }
}

fn find_merge_requests() {
    if let Some(mrs) =
        run_glab::<Vec<MergeRequest>>(&["mr", "list", "-R", "gitlab.com/glab-env/glab"])
    {
        for mr in &mrs {
            println!("{} {} ({})", mr.iid, mr.title, mr.state);
        }
    }
}

fn is_glab_installed() -> bool {
    Command::new("glab")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s: ExitStatus| s.success())
        .unwrap_or(false)
}

/// Runs `glab` with the given arguments, automatically appending `-F json` to
/// request JSON output, then deserializes stdout into `T`.
///
/// Returns `None` and prints an error message if the process fails to launch,
/// exits with a non-zero status, or the output cannot be deserialized into `T`.
fn run_glab<T>(args: &[&str]) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut full_args = args.to_vec();
    full_args.extend_from_slice(&["-F", "json"]);

    let output = match Command::new("glab").args(&full_args).output() {
        Err(e) => {
            println!("Error: failed to run glab: {}", e);
            return None;
        }
        Ok(o) => o,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Error: glab exited with {}: {}", output.status, stderr);
        return None;
    }

    match serde_json::from_slice::<T>(&output.stdout) {
        Ok(value) => Some(value),
        Err(e) => {
            println!("Error: failed to parse glab output as JSON: {}", e);
            None
        }
    }
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
