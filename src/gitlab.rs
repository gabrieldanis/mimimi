//! Utilities for invoking the `glab` CLI and parsing its output.

use std::process::{Command, ExitStatus};

use serde::Deserialize;

/// Returns `true` if `glab` is present and executable on `PATH`.
pub fn is_glab_installed() {
    println!("Checking if glab is installed...");
    let result = Command::new("glab")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s: ExitStatus| s.success())
        .unwrap_or(false);
    if !result {
        eprintln!("glab is not installed. Install it from https://gitlab.com/gitlab-org/cli");
        std::process::exit(1);
    }
}

/// Runs `glab` with the given arguments, automatically appending `-F json` to
/// request JSON output, then deserializes stdout into `T`.
///
/// Returns `None` and prints an error message if the process fails to launch,
/// exits with a non-zero status, or the output cannot be deserialized into `T`.
pub fn run_glab<T>(args: &[&str]) -> Option<T>
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
