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
- **15 built-in tools** — `read_file`, `write_file`, `edit_file`,
  `list_directory`, `bash`, `glob`, `grep`, `web_fetch`, `ask_user`,
  `apply_patch`, `search` (code index), `terminal`, `git_status`, `git_diff`,
  `git_commit`.
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
- **Interactive TUI** — a line-based chat REPL with approval prompts.
- **Notifications** — completion alerts (macOS native).
- **Hooks & plugins** — before/after tool hooks and a plugin bundle system.
- **Cron/automations** — an interval-based job scheduler.
- **Code/PR review** — heuristic diff review.
- **MCP** — a minimal Model Context Protocol client over stdio.
- **Browser automation** — CDP headless Chrome control, including WebSocket
  `Runtime.evaluate`.
- **Computer/desktop use** — screenshot + coordinate control (macOS).
- **SSH** — run commands on a remote host.
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
