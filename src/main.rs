mod app;
mod diff;
mod gitlab;
mod highlight;
mod opencode;
mod types;

use std::error::Error;

use app::App;
use gitlab::GitLabClient;

fn main() -> Result<(), Box<dyn Error>> {
    let client = GitLabClient::from_git_remote()?;
    ratatui::run(|terminal| App::new(client.clone()).run(terminal))?;

    Ok(())
}
