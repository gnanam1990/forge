# forge — Roadmap

forge is an original, self-contained coding agent written in Rust. This
document maps the full set of modules a production coding agent needs, and
tracks what is built. Each module is built, tested, committed, and pushed
independently.

Legend: ✅ built · ⬜ planned

## Core
- ✅ **Agent loop** — provider turns, tool execution, turn budget
- ✅ **Tool system** — `Tool` trait, `Registry`, workspace-boundary enforcement
- ✅ **Config** — JSON config, env overrides
- ✅ **CLI** — `run`, `tools`, `orchestrate`, `workflow`, `resume`, `sessions`,
  `chat`, `memory`, `cron`, `review`, `mcp`, `browser`, `desktop`, `init`
- ✅ **Providers** — OpenAI-compatible HTTP + scriptable mock

## Tools
- ✅ `read_file`, `write_file`, `edit_file`, `list_directory`
- ✅ `bash`, `glob`, `grep`, `web_fetch`, `ask_user`
- ✅ `apply_patch`, `search` (code index), `terminal`
- ✅ `git_status`, `git_diff`, `git_commit`

## Orchestration
- ✅ **Parallel sub-agents** — `run_parallel`, `forge orchestrate`
- ✅ **Workflow engine** — DAG, phases, budget-aware fan-out, `forge workflow`
- ✅ **Stall watchdog** — timeout detection + bounded retries

## Sessions & state
- ✅ **Sessions & resume** — persist/load conversation, resume across runs
- ✅ **Context management** — token budget, compaction/summarization
- ✅ **Memory** — cross-session fact store, `forge memory`

## Safety & permissions
- ✅ **Permission system** — tool safety levels, allow/deny rules, approval flow
- ✅ **Sandbox** — command isolation (minimal env, macOS sandbox-exec)

## Interface
- ✅ **TUI** — interactive chat REPL
- ✅ **Notifications** — completion alerts (macOS native)

## Integration
- ✅ **Git** — status/diff/commit helpers
- ✅ **MCP** — minimal Model Context Protocol client over stdio
- ✅ **Hooks & plugins** — before/after tool hooks, plugin bundle system
- ✅ **Code/PR review** — heuristic diff review, `forge review`
- ✅ **Cron/automations** — interval-based job scheduler, `forge cron`

## Local control
- ✅ **Browser automation** — CDP headless Chrome, `forge browser`
- ✅ **Computer/desktop use** — screenshot + coordinate control, `forge desktop`
- ⬜ **Terminal/SSH** — remote sessions (foundation: `terminal` tool)

## Status
All core modules are built. Remaining stretch: full WebSocket CDP evaluation,
model-based summarization, and remote/SSH terminal sessions.
