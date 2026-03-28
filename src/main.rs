use std::{
    io,
    process::{Command, ExitStatus},
};

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

    println!(
        "To list the comments of a merge request enter the number on the left of it without the exclamation mark"
    );

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
}

fn find_merge_requests() {
    let output = Command::new("glab")
        .arg("mr")
        .arg("list")
        .arg("-R")
        .arg("gitlab.com/glab-env/glab")
        .output()
        .expect("failed to execute process");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Error: {}", stderr);
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
