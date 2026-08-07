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
- **Built-in tools**:
  - `read_file` — read a text file inside the workspace
  - `write_file` — write a text file inside the workspace
  - `edit_file` — replace text inside a file
  - `list_directory` — list a directory's entries
  - `bash` — run a shell command with a bounded timeout
  - `glob` — find files by glob pattern
  - `grep` — search file contents by regex
  - `web_fetch` — fetch a URL and return its text
  - `ask_user` — ask the user a question
- **Sub-agent orchestration** — run several prompts as independent sub-agents
  in parallel, each with its own context, sharing the provider.
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
