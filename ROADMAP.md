# forge — Roadmap

forge is an original, self-contained coding agent written in Rust. This
document maps the full set of modules a production coding agent needs, and
tracks what is built. Each module is built, tested, committed, and pushed
independently.

Legend: ✅ built · 🔨 in progress · ⬜ planned

## Core
- ✅ **Agent loop** — provider turns, tool execution, turn budget
- ✅ **Tool system** — `Tool` trait, `Registry`, workspace-boundary enforcement
- ✅ **Config** — JSON config, env overrides
- ✅ **CLI** — `run`, `tools`, `orchestrate`, `init`
- ✅ **Providers** — OpenAI-compatible HTTP + scriptable mock

## Tools
- ✅ `read_file`, `write_file`, `edit_file`, `list_directory`
- ✅ `bash`, `glob`, `grep`, `web_fetch`, `ask_user`
- ⬜ `apply_patch`, `search` (code index), `terminal` (virtual terminal)

## Orchestration
- ✅ **Parallel sub-agents** — `run_parallel`, `forge orchestrate`
- ⬜ **Workflow engine** — phases, pipelines, dependency DAG, budget-aware fan-out
- ⬜ **Stall watchdog** — silence-keyed detection + bounded retries

## Sessions & state
- ✅ **Sessions & resume** — persist/load conversation, resume across runs
- ✅ **Context management** — token budget, compaction/summarization
- ⬜ **Memory** — cross-session learned facts

## Safety & permissions
- ✅ **Permission system** — tool safety levels, allow/deny rules, approval flow
- ⬜ **Sandbox** — OS-level command isolation

## Interface
- ✅ **TUI** — interactive terminal UI
- ⬜ **Notifications** — completion alerts

## Integration
- ✅ **Git** — status/diff/commit helpers
- ⬜ **MCP** — Model Context Protocol client/server
- ⬜ **Hooks & plugins** — lifecycle hooks, plugin loading
- ⬜ **Code/PR review** — review workflows
- ⬜ **Cron/automations** — scheduled tasks

## Local control
- ⬜ **Browser automation** — CDP-based browser control
- ⬜ **Computer/desktop use** — screenshot + coordinate control
- ⬜ **Terminal/SSH** — remote and virtual terminal control

## Status
Current milestone: sessions & resume, permissions, context management, TUI,
and git integration. See commit history for what has shipped.
