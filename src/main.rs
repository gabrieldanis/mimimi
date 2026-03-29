mod gitlab;
mod tui;
mod types;

use std::io;
use tui::App;

use crate::gitlab::{is_glab_installed, run_glab};
use crate::types::{MergeRequest, MergeRequestWithDiscussions};

fn main() {
    is_glab_installed();
    // TODO: add is_glab_logged_in checker
    let selected_mr = merge_request_loop();

    print_merge_request_comments(selected_mr);
}

fn merge_request_loop() -> i32 {
    let mut selected = 0;
    while selected == 0 {
        print_merge_requests();
        selected = select_merge_request();
    }
    selected
}

fn select_merge_request() -> i32 {
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
    number
}

fn print_merge_request_comments(selected_mr: i32) {
    if let Some(mr) = run_glab::<MergeRequestWithDiscussions>(&[
        "-R",
        "gitlab.com/glab-env/glab",
        "mr",
        "view",
        &selected_mr.to_string(),
        "--comments",
    ]) {
        for discussion in &mr.discussions {
            for note in &discussion.notes {
                println!("{}: {}", note.author.username, note.body);
            }
        }
    }
}

fn print_merge_requests() {
    let _ = ratatui::run(|terminal| App::default().run(terminal));
    // for mr in &mrs {
    //     println!("{} {} ({})", mr.iid, mr.title, mr.state);
    // }
}
