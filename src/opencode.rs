//! Integration with OpenCode: port discovery, HTTP prompt sending, and
//! new-window launch fallback.

use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Attempt to discover the port of a running `opencode --port` instance.
///
/// On Linux, uses `pgrep` + `lsof`. On Windows, uses `tasklist` + `netstat`.
/// If multiple ports are found, picks the one whose `GET /path` matches
/// the current working directory. Otherwise returns the first.
///
/// Returns `None` when no running instance can be found.
pub fn discover_opencode_port() -> Option<u16> {
    let ports = if cfg!(target_os = "windows") {
        discover_ports_windows()?
    } else {
        discover_ports_unix()?
    };

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

// ---------------------------------------------------------------------------
// Unix port discovery (pgrep + lsof)
// ---------------------------------------------------------------------------

/// Discover opencode listening ports on Linux/macOS via `pgrep` + `lsof`.
fn discover_ports_unix() -> Option<Vec<u16>> {
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
            let addr_field = line.split_whitespace().nth(8)?;
            let port_str = addr_field.rsplit(':').next()?;
            port_str.parse::<u16>().ok()
        })
        .collect();

    Some(ports)
}

// ---------------------------------------------------------------------------
// Windows port discovery (tasklist + netstat)
// ---------------------------------------------------------------------------

/// Discover opencode listening ports on Windows via `tasklist` + `netstat`.
fn discover_ports_windows() -> Option<Vec<u16>> {
    // Step 1: Find opencode.exe PIDs using tasklist.
    //   tasklist /FI "IMAGENAME eq opencode.exe" /FO CSV /NH
    // Output lines look like: "opencode.exe","12345","Console","1","12,345 K"
    let tasklist_output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq opencode.exe", "/FO", "CSV", "/NH"])
        .output()
        .ok()?;

    if !tasklist_output.status.success() {
        return None;
    }

    let tasklist_text = String::from_utf8_lossy(&tasklist_output.stdout);
    let pids: Vec<u32> = tasklist_text
        .lines()
        .filter(|line| line.contains("opencode.exe"))
        .filter_map(|line| {
            // Second CSV field is the PID (quoted).
            let pid_field = line.split(',').nth(1)?;
            let pid_str = pid_field.trim_matches('"');
            pid_str.parse::<u32>().ok()
        })
        .collect();

    if pids.is_empty() {
        return None;
    }

    // Step 2: Find listening TCP ports using netstat.
    //   netstat -ano
    // Output lines look like:
    //   TCP    127.0.0.1:3456    0.0.0.0:0    LISTENING    12345
    // Note: the status column is locale-dependent (e.g. "ABHÖREN" on German
    // Windows), so we don't filter on it. PID matching + the later HTTP check
    // are sufficient.
    let netstat_output = Command::new("netstat").args(["-ano"]).output().ok()?;

    if !netstat_output.status.success() {
        return None;
    }

    let netstat_text = String::from_utf8_lossy(&netstat_output.stdout);
    let ports: Vec<u16> = netstat_text
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                return None;
            }
            if !parts[0].eq_ignore_ascii_case("TCP") {
                return None;
            }
            // Match by PID (last column).
            let pid: u32 = parts.last()?.parse().ok()?;
            if !pids.contains(&pid) {
                return None;
            }
            // Only consider entries where the foreign address is 0.0.0.0:0
            // (i.e. listening sockets, not established connections).
            if parts[2] != "0.0.0.0:0" && parts[2] != "[::]:0" {
                return None;
            }
            // Extract port from local address (e.g. "127.0.0.1:3456" or "[::1]:3456")
            let addr = parts[1];
            let port_str = addr.rsplit(':').next()?;
            port_str.parse::<u16>().ok()
        })
        .collect();

    Some(ports)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// `GET /path` on a running OpenCode instance. Returns the working directory.
fn fetch_path(port: u16) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/path");
    let body = ureq::get(&url)
        .call()
        .map_err(|e| format!("GET /path failed: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("read body failed: {e}"))?;

    // The endpoint returns JSON like {"worktree": "C:\\work\\...", "directory": "..."}.
    // Extract the "directory" field as the working directory.
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body)
        && let Some(dir) = json.get("directory").and_then(|v| v.as_str())
    {
        return Ok(dir.to_string());
    }

    // Fallback: return raw body trimmed.
    Ok(body.trim().to_string())
}

/// Send a prompt to a running OpenCode instance via its HTTP API.
///
/// 1. Clear the current prompt
/// 2. Append the new prompt text
/// 3. Submit the prompt
pub fn send_prompt_http(port: u16, prompt: &str) -> Result<(), String> {
    let base = format!("http://127.0.0.1:{port}");

    // Step 1: clear prompt
    ureq::post(&format!("{base}/tui/clear-prompt"))
        .header("Content-Type", "application/json")
        .send(b"{}")
        .map_err(|e| format!("POST /tui/clear-prompt failed: {e}"))?;

    // Small pause so the TUI processes the clear before we append.
    thread::sleep(Duration::from_millis(100));

    // Step 2: append prompt text
    let body = serde_json::json!({ "text": prompt });
    let body_str =
        serde_json::to_string(&body).map_err(|e| format!("JSON serialization failed: {e}"))?;
    ureq::post(&format!("{base}/tui/append-prompt"))
        .header("Content-Type", "application/json")
        .send(body_str.as_bytes())
        .map_err(|e| format!("POST /tui/append-prompt failed: {e}"))?;

    // Small pause so the TUI processes the append before we submit.
    thread::sleep(Duration::from_millis(100));

    // Step 3: submit
    ureq::post(&format!("{base}/tui/submit-prompt"))
        .header("Content-Type", "application/json")
        .send(b"{}")
        .map_err(|e| format!("POST /tui/submit-prompt failed: {e}"))?;

    Ok(())
}

/// Launch `opencode --port <port>` in a **new terminal window**, wait for
/// it to become ready, then send the prompt via HTTP.
///
/// Returns `Ok(())` on success or an `Err` describing what went wrong.
pub fn launch_and_send(prompt: &str) -> Result<(), String> {
    let port = pick_available_port();

    launch_in_new_window(port)?;

    poll_until_ready(port)?;

    // The HTTP server is up but the TUI input may not be fully initialised yet.
    thread::sleep(Duration::from_secs(2));

    send_prompt_http(port, prompt)
}

/// Pick a port for the new opencode instance.
///
/// Uses a simple strategy: bind to port 0, read the assigned port, close the
/// socket. There's a small TOCTOU window but it's good enough.
fn pick_available_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(14321) // unlikely fallback
}

/// Spawn `opencode --port <port>` in a new terminal window.
fn launch_in_new_window(port: u16) -> Result<(), String> {
    let port_str = port.to_string();

    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", &format!("start cmd /K opencode --port {port_str}")])
            .spawn()
            .map_err(|e| format!("Failed to open new window: {e}"))?;
    } else {
        // Try common Linux terminal emulators in order of likelihood.
        let terminals = [
            ("x-terminal-emulator", vec!["-e"]),
            ("gnome-terminal", vec!["--"]),
            ("konsole", vec!["-e"]),
            ("xfce4-terminal", vec!["-e"]),
            ("xterm", vec!["-e"]),
        ];

        let mut launched = false;
        for (term, prefix_args) in &terminals {
            let mut args: Vec<&str> = prefix_args.to_vec();
            args.extend_from_slice(&["opencode", "--port", &port_str]);

            if Command::new(term).args(&args).spawn().is_ok() {
                launched = true;
                break;
            }
        }

        if !launched {
            return Err("Could not find a terminal emulator to launch opencode".into());
        }
    }

    Ok(())
}

/// Poll `GET /path` on the given port until it responds, with a timeout.
fn poll_until_ready(port: u16) -> Result<(), String> {
    let timeout = Duration::from_secs(15);
    let interval = Duration::from_millis(500);
    let start = Instant::now();

    while start.elapsed() < timeout {
        if fetch_path(port).is_ok() {
            return Ok(());
        }
        thread::sleep(interval);
    }

    Err(format!(
        "opencode on port {port} did not become ready within {timeout:?}"
    ))
}
