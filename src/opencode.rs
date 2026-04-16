//! Integration with OpenCode: port discovery, HTTP prompt sending, and
//! interactive fallback via `opencode run`.

use std::process::Command;

/// Attempt to discover the port of a running `opencode --port` instance.
///
/// 1. `pgrep -f "opencode .*--port"` to find candidate PIDs.
/// 2. `lsof -iTCP -sTCP:LISTEN -p <pids>` to find the listening port.
/// 3. If multiple ports found, pick the one whose `GET /path` matches
///    the current working directory. Otherwise return the first.
///
/// Returns `None` when no running instance can be found.
pub fn discover_opencode_port() -> Option<u16> {
    let pgrep_output = Command::new("pgrep")
        .args(["-f", "opencode .*--port"])
        .output()
        .ok()?;

    if !pgrep_output.status.success() {
        return None;
    }

    let pids: Vec<&str> = std::str::from_utf8(&pgrep_output.stdout)
        .ok()?
        .lines()
        .filter(|l| !l.is_empty())
        .collect();

    if pids.is_empty() {
        return None;
    }

    let pid_list = pids.join(",");
    let lsof_output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-P", "-n", "-p", &pid_list])
        .output()
        .ok()?;

    if !lsof_output.status.success() {
        return None;
    }

    let lsof_text = std::str::from_utf8(&lsof_output.stdout).ok()?;
    let ports: Vec<u16> = lsof_text
        .lines()
        .filter_map(|line| {
            // Lines look like: node 12345 user 10u IPv4 ... TCP 127.0.0.1:3456 (LISTEN)
            // We want the port number after the last ':' in the address field.
            let addr_field = line.split_whitespace().nth(8)?;
            let port_str = addr_field.rsplit(':').next()?;
            port_str.parse::<u16>().ok()
        })
        .collect();

    if ports.is_empty() {
        return None;
    }

    if ports.len() == 1 {
        return Some(ports[0]);
    }

    // Multiple instances: pick the one whose working directory matches ours.
    let cwd = std::env::current_dir().ok()?;
    for &port in &ports {
        if let Ok(body) = fetch_path(port)
            && std::path::Path::new(body.trim()) == cwd
        {
            return Some(port);
        }
    }

    // Fallback: return the first port found.
    Some(ports[0])
}

/// `GET /path` on a running OpenCode instance. Returns the working directory.
fn fetch_path(port: u16) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/path");
    let body = ureq::get(&url)
        .call()
        .map_err(|e| format!("GET /path failed: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("read body failed: {e}"))?;
    Ok(body)
}

/// Send a prompt to a running OpenCode instance via its HTTP API.
///
/// Fires three sequential POSTs to `/tui/publish`:
/// 1. Clear the current prompt
/// 2. Append the new prompt text
/// 3. Submit the prompt
pub fn send_prompt_http(port: u16, prompt: &str) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/tui/publish");

    let payloads = [
        serde_json::json!({
            "type": "tui.command.execute",
            "properties": { "command": "prompt.clear" }
        }),
        serde_json::json!({
            "type": "tui.prompt.append",
            "properties": { "text": prompt }
        }),
        serde_json::json!({
            "type": "tui.command.execute",
            "properties": { "command": "prompt.submit" }
        }),
    ];

    for payload in &payloads {
        let body = serde_json::to_string(payload)
            .map_err(|e| format!("JSON serialization failed: {e}"))?;

        ureq::post(&url)
            .header("Content-Type", "application/json")
            .send(body.as_bytes())
            .map_err(|e| format!("POST /tui/publish failed: {e}"))?;
    }

    Ok(())
}

/// Launch `opencode run "<prompt>"` as an interactive child process.
///
/// This takes over the terminal. When the user exits OpenCode, control returns
/// to the caller.
pub fn run_opencode_interactive(prompt: &str) -> Result<(), String> {
    let mut child = Command::new("opencode")
        .args(["run", prompt])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn opencode: {e}"))?;

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for opencode: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("opencode exited with status: {status}"))
    }
}
