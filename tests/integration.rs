//! Integration tests that drive the agent loop and the tool system end to end
//! with the scriptable mock provider.

use forge::agent::mock::{call, read_call, MockProvider, ScriptTurn};
use forge::agent::Agent;
use forge::tools::{Registry, ToolContext};
use serde_json::json;

fn agent_with(script: Vec<ScriptTurn>) -> Agent {
    Agent::new(Box::new(MockProvider::new(script)), Registry::builtin(), 10)
}

#[test]
fn agent_answers_without_tools() {
    let agent = agent_with(vec![ScriptTurn::Answer("hello".into())]);
    let outcome = agent.run("hi").unwrap();
    assert_eq!(outcome.final_text, "hello");
    assert_eq!(outcome.tool_calls, 0);
    assert_eq!(outcome.turns, 1);
}

#[test]
fn agent_executes_a_tool_call() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "forge content").unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let agent = agent_with(vec![
        ScriptTurn::Tools(vec![read_call("call_1", "a.txt")]),
        ScriptTurn::Answer("read it".into()),
    ]);
    let outcome = agent.run("read a.txt").unwrap();
    assert_eq!(outcome.tool_calls, 1);
    assert_eq!(outcome.final_text, "read it");
}

#[test]
fn agent_handles_unknown_tool() {
    let agent = agent_with(vec![
        ScriptTurn::Tools(vec![call("c1", "does_not_exist", json!({}))]),
        ScriptTurn::Answer("ok".into()),
    ]);
    let outcome = agent.run("go").unwrap();
    assert_eq!(outcome.tool_calls, 1);
    assert_eq!(outcome.final_text, "ok");
}

#[test]
fn agent_stops_at_max_turns() {
    // A script that keeps calling tools forever; the loop must stop at the budget.
    let agent = Agent::new(
        Box::new(MockProvider::new(vec![
            ScriptTurn::Tools(vec![call("c1", "glob", json!({"pattern": "*.rs"}))]),
            ScriptTurn::Tools(vec![call("c2", "glob", json!({"pattern": "*.rs"}))]),
        ])),
        Registry::builtin(),
        2,
    );
    let err = agent.run("loop").unwrap_err();
    assert!(err.to_string().contains("max turns"));
}

#[test]
fn write_then_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();

    let write = registry.get("write_file").unwrap();
    let res = write
        .run(&json!({"path": "sub/out.txt", "content": "hello"}), &ctx)
        .unwrap();
    assert!(res.ok);

    let read = registry.get("read_file").unwrap();
    let res = read.run(&json!({"path": "sub/out.txt"}), &ctx).unwrap();
    assert!(res.ok);
    assert_eq!(res.output, "hello");
}

#[test]
fn read_rejects_outside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let read = registry.get("read_file").unwrap();
    let res = read.run(&json!({"path": "/etc/passwd"}), &ctx).unwrap();
    assert!(!res.ok);
    assert!(res.output.contains("outside the workspace"));
}

#[test]
fn bash_runs_a_command() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let bash = registry.get("bash").unwrap();
    let res = bash
        .run(&json!({"command": "echo forge-ok"}), &ctx)
        .unwrap();
    assert!(res.ok);
    assert!(res.output.contains("forge-ok"));
}

#[test]
fn glob_finds_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main(){}").unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn x(){}").unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let glob = registry.get("glob").unwrap();
    let res = glob.run(&json!({"pattern": "*.rs"}), &ctx).unwrap();
    assert!(res.ok);
    assert!(res.output.contains("main.rs"));
    assert!(res.output.contains("lib.rs"));
}

#[test]
fn grep_finds_matching_lines() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "alpha\nbeta\ngamma\n").unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let grep = registry.get("grep").unwrap();
    let res = grep.run(&json!({"pattern": "beta"}), &ctx).unwrap();
    assert!(res.ok);
    assert!(res.output.contains("a.txt:2:beta"));
}
