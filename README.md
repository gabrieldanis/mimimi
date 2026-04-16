# mimimi

A terminal UI for reviewing GitLab merge request comments and forwarding them to [OpenCode](https://opencode.ai) for AI-assisted analysis.

The name is a nod to the German word for whining — because dealing with review comments can feel exactly like that.

## What it does

mimimi connects to your GitLab project (auto-detected from the git remote), fetches open merge requests, and lets you browse their review threads directly in your terminal. For any thread or batch of threads you select, it builds a structured prompt — including the surrounding diff context — and sends it to a running OpenCode instance so you can immediately start working through the feedback with AI assistance.

You can also mark threads as resolved directly from the TUI without switching to the browser.

## Features

- Lists open MRs for the current project
- Displays review threads with syntax-highlighted diff context
- Shows full thread conversations (root comment + all replies stacked)
- Multi-select threads with Space / select all with `a`
- Send one or more threads as a batch prompt to OpenCode (`Enter`)
- Mark selected threads as resolved on GitLab (`r`)
- Discovers a running OpenCode instance automatically, or launches one
- Stores your GitLab token in the OS keyring so you only enter it once

## Requirements

- Rust 1.85+ (edition 2024)
- A GitLab personal access token with `api` scope
- A git repository with a GitLab remote named `origin`
- [OpenCode](https://opencode.ai) installed (optional — mimimi will launch it if needed)

## Installation

```bash
git clone <this repo>
cd mimimi
cargo build --release
# install globally
cargo install --path .
```

## Usage

Run mimimi from inside any git repository that has a GitLab remote:

```bash
mimimi
```

On first run you will be prompted for a GitLab personal access token. It is stored in the OS keyring (Windows Credential Manager, macOS Keychain, or the system's secret service on Linux) and reused on subsequent runs. You can also set the `GITLAB_TOKEN` environment variable to skip the prompt.

### Key bindings

**MR list**

| Key         | Action          |
| ----------- | --------------- |
| `j` / `↓`   | Move down       |
| `k` / `↑`   | Move up         |
| `Enter`     | Open MR threads |
| `q` / `Esc` | Quit            |

**Thread view**

| Key         | Action                                                  |
| ----------- | ------------------------------------------------------- |
| `j` / `↓`   | Next thread                                             |
| `k` / `↑`   | Previous thread                                         |
| `Space`     | Toggle thread selection                                 |
| `a`         | Select all / deselect all                               |
| `Enter`     | Send selected thread(s) to OpenCode                     |
| `r`         | Mark selected thread(s) as resolved (with confirmation) |
| `q` / `Esc` | Back to MR list                                         |

When no threads are selected, `Enter` and `r` operate on the currently focused thread.

## Authentication

Token resolution order:

1. `GITLAB_TOKEN` environment variable
2. OS keyring (set on first interactive login)
3. Interactive prompt (stored in keyring for next time)

To update a stored token, unset it from the keyring using your OS credential manager and re-run mimimi.

## Building from source

```bash
cargo build            # debug
cargo build --release  # optimised
```
