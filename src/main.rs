mod app;
mod gitlab;
mod types;

use app::App;
use std::error::Error;

use crate::gitlab::is_glab_installed;

fn main() -> Result<(), Box<dyn Error>> {
    is_glab_installed();
    // TODO: add is_glab_logged_in checker
    ratatui::run(|terminal| App::default().run(terminal))?;

    Ok(())
}

// fn print_merge_request_comments(selected_mr: i32) {
//     if let Some(mr) = run_glab::<MergeRequestWithDiscussions>(&[
//         "-R",
//         "gitlab.com/glab-env/glab",
//         "mr",
//         "view",
//         &selected_mr.to_string(),
//         "--comments",
//     ]) {
//         for discussion in &mr.discussions {
//             for note in &discussion.notes {
//                 println!("{}: {}", note.author.username, note.body);
//             }
//         }
//     }
// }
