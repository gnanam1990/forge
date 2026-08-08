# forge

**forge** is an original, self-contained coding agent written in Rust. It is a
from-scratch implementation — a small agent loop, a tool system, and a CLI —
built to be simple, testable, and easy to extend.

> forge is an independent project. It reimplements common coding-agent ideas
> (file tools, a shell tool, an agent loop) from scratch; it does not contain
> code from any commercial product.

## Features

- **Agent loop** — a provider produces assistant turns; the loop executes any
  tool calls the assistant requests and feeds the results back, until the
  assistant stops or the turn budget is exhausted.
- **Tool system** — a `Tool` trait, a shared `ToolContext`, and a `Registry`.
  Every tool enforces the workspace boundary.
- **15+ built-in tools** — `read_file`, `write_file`, `edit_file`,
  `list_directory`, `bash`, `glob`, `grep`, `web_fetch`, `web_search`,
  `ask_user`, `apply_patch` (built-in parser), `search` (persistent code index),
  `terminal` (persistent sessions), `git_status`, `git_diff`, `git_commit`,
  `ssh`, `remember`, `recall`.
- **Orchestration** — parallel sub-agents, a DAG workflow engine with phases
  and budget-aware fan-out, and a stall watchdog with bounded retries.
- **Sessions & resume** — persist conversations to disk and resume them across
  runs.
- **Permissions** — tools declare a safety level; a policy can allow/deny tools;
  prompt-level tools ask an approver before running.
- **Context management** — a token budget and a compactor that folds old
  messages into a summary when the conversation grows too large.
- **Memory** — a durable cross-session fact store.
- **Sandbox** — command isolation with a minimal environment.
- **Interactive TUI** — a full-screen terminal UI (ratatui) with a message
  list, input box, and status bar.
- **Notifications** — completion alerts (macOS native).
- **Hooks & plugins** — before/after tool hooks and a plugin bundle system.
- **Cron/automations** — an interval-based job scheduler.
- **Code/PR review** — heuristic diff review, plus a model-based reviewer with
  heuristic fallback.
- **MCP** — a Model Context Protocol client over stdio, with MCP tools
  registered into the agent's tool registry, plus resources, prompts, ping,
  notifications, sampling, logging, roots, and completions support.
- **`forge config` / `forge config-set <key> <value>`** — show or edit config.
- **`forge session delete <id>`** — delete a session.
- **`forge memory clear`** — clear the memory store.
- **`forge telemetry <on|off>`** — toggle telemetry.
- **`forge alias <name> <command>`** — manage command aliases.
- **`forge plugins`** — lists plugins loaded from the plugins directory.
- **Session listing** — `forge sessions` shows message counts and timestamps.
- **Search index invalidation** — the code index rebuilds when sources change.
- **Plugins** — a plugin bundle system, a plugin registry that tracks and
  enables/disables plugins, a `forge plugin` CLI (list/enable/disable/add), plus
  loading command-backed plugins from a directory of JSON files.
- **Telemetry** — a minimal usage tracker that appends JSON events to a file.
- **Memory** — a durable cross-session fact store, wired into the agent loop via
  `remember`/`recall` tools.
- **Browser automation** — CDP headless Chrome with navigate, back, forward,
  reload, click, type, screenshot, get_text, get_title, get_url, get_html,
  get_cookies, set_cookie, get/set_local_storage, get_performance, scroll,
  wait_for_load, wait_for_selector, get_element, and `Runtime.evaluate` over
  the DevTools WebSocket.
- **Computer/desktop use** — screenshot + coordinate control (macOS).
- **SSH** — run commands on a remote host.
- **Sandbox** — command isolation with configurable network/file restrictions.
- **Logging** — a small structured logger.
- **Config validation** — validates workspace, provider, and turn settings.
- **Config-driven wiring** — auto-loads MCP servers, plugins, hooks, and
  telemetry from config at startup.
- **Telemetry** — a minimal usage tracker recording run/plan/workflow events.
- **Distinct exit codes** — config=2, provider=3, tool=4, args=5, agent=6.
- **`forge doctor`** — checks the environment (config, provider, git, browser,
  desktop).
- **`forge info`** — prints version, workspace, model, tool and session counts.
- **`forge version` / `forge models` / `forge help` / `forge docs`** — version,
  model catalog, command summary, and a quick-start guide.
- **`forge stats` / `forge env`** — usage stats from telemetry and environment
  info.
- **`forge shell`** — an interactive shell session (real pty).
- **`forge watch <dir> <cmd>`** — run a command when files change.
- **`forge benchmark`** — a simple tool benchmark.
- **`forge provider`** — show, set, list, or add providers (multi-model support).
- **`forge plugin docs`** — a guide to the plugin format and commands.
- **TUI command palette** — `/help`, `/sessions`, `/exit` inside the chat UI.
- **`forge run --file`** — run a prompt read from a file.
- **Session export/import** — `forge export <id> <path>` and `forge import <path>`.
- **`forge init`** — scaffolds a project (config, README, src, .forge).
- **TUI polish** — scrollable message list (PageUp/PageDown) and input history
  (Up/Down arrows).
- **Real pty terminal** — persistent terminal sessions backed by a real
  pseudo-terminal, so interactive programs work.
- **Fuzzy search** — the code index falls back to subsequence matching.
- **Model-based summarization** — context compaction asks the provider to
  summarize, with a heuristic fallback.
- **`forge setup`** — writes a working config plus sample workflow and cron
  files so you can start immediately.
- **Providers** — an OpenAI-compatible HTTP provider, plus a scriptable mock
  provider for tests and demos.
- **Config** — a small JSON config file (`~/.config/forge/config.json`).

## Build

```sh
cargo build --release
```

## Usage

```sh
# List the available tools
forge tools

# Write a sample config
forge init

# Run the agent on a prompt (needs a configured provider)
forge run "summarize this project"

# Run several prompts as parallel sub-agents (file is a JSON array of strings)
forge orchestrate prompts.json

# Start an interactive chat session
forge chat

# Resume a saved session
forge resume <session-id> "continue the work"

# List saved sessions
forge sessions

# One-command setup: writes config + sample workflow + sample cron
forge setup
```

### Configuration

`forge init` writes a sample config to `~/.config/forge/config.json` (override
the location with `FORGE_CONFIG`):

```json
{
  "workspace": "/path/to/your/project",
  "provider": {
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
    "api_key": ""
  },
  "max_turns": 10
}
```

The API key can also be set with the `FORGE_API_KEY` environment variable.

## Test

```sh
cargo test
```

The suite covers the agent loop (with the mock provider), every tool, and the
workspace-boundary enforcement.

## Project layout

```
src/
  main.rs        CLI entry point
  lib.rs         library root
  cli.rs         argument parsing and dispatch
  config.rs      configuration loading
  error.rs       unified error type
  agent/         agent loop, provider trait, mock + HTTP providers
  tools/         Tool trait, registry, and the built-in tools
tests/
  integration.rs end-to-end tests
```

## License

MIT — see [LICENSE](LICENSE).
