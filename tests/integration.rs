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

    let agent = agent_with(vec![
        ScriptTurn::Tools(vec![read_call("call_1", "a.txt")]),
        ScriptTurn::Answer("read it".into()),
    ])
    .with_workspace(dir.path());
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

#[test]
fn edit_file_replaces_text() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello world").unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let edit = registry.get("edit_file").unwrap();
    let res = edit
        .run(
            &json!({"path": "a.txt", "old": "world", "new": "forge"}),
            &ctx,
        )
        .unwrap();
    assert!(res.ok);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "hello forge"
    );
}

#[test]
fn edit_file_reports_missing_pattern() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let edit = registry.get("edit_file").unwrap();
    let res = edit
        .run(&json!({"path": "a.txt", "old": "nope", "new": "x"}), &ctx)
        .unwrap();
    assert!(!res.ok);
    assert!(res.output.contains("not found"));
}

#[test]
fn list_directory_lists_entries() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "x").unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let list = registry.get("list_directory").unwrap();
    let res = list.run(&json!({}), &ctx).unwrap();
    assert!(res.ok);
    assert!(res.output.contains("file\ta.txt"));
    assert!(res.output.contains("dir\tsub"));
}

#[test]
fn ask_user_uses_responder() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    // Replace the default ask_user with a fixed responder for the test.
    let mut registry = registry;
    registry.register(Box::new(
        forge::tools::ask_user::AskUserTool::with_responder(Box::new(|_| "42".to_string())),
    ));
    let ask = registry.get("ask_user").unwrap();
    let res = ask.run(&json!({"question": "what is 6*7?"}), &ctx).unwrap();
    assert!(res.ok);
    assert_eq!(res.output, "42");
}

#[test]
fn web_fetch_requires_url() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let fetch = registry.get("web_fetch").unwrap();
    let res = fetch.run(&json!({}), &ctx).unwrap_err();
    assert!(res.to_string().contains("url"));
}

#[test]
fn run_parallel_returns_results_in_order() {
    use forge::agent::mock::ScriptTurn;
    let agent = Agent::new(
        Box::new(MockProvider::new(vec![
            ScriptTurn::Answer("one".into()),
            ScriptTurn::Answer("two".into()),
            ScriptTurn::Answer("three".into()),
        ])),
        Registry::builtin(),
        5,
    );
    let mut results = agent
        .run_parallel(&["a".into(), "b".into(), "c".into()])
        .unwrap();
    // The mock provider is a single shared script consumed by the parallel
    // threads, so completion order is not deterministic — compare as a set.
    results.sort();
    assert_eq!(results, vec!["one", "three", "two"]);
}

#[test]
fn run_into_resumes_a_conversation() {
    use forge::agent::mock::ScriptTurn;
    let agent = Agent::new(
        Box::new(MockProvider::new(vec![
            ScriptTurn::Answer("first".into()),
            ScriptTurn::Answer("second".into()),
        ])),
        Registry::builtin(),
        5,
    );
    let mut messages = vec![forge::agent::Message::System("sys".into())];
    let first = agent.run_into(&mut messages, "q1").unwrap();
    assert_eq!(first.final_text, "first");
    assert_eq!(messages.len(), 3); // system + user + assistant
    let second = agent.run_into(&mut messages, "q2").unwrap();
    assert_eq!(second.final_text, "second");
    assert_eq!(messages.len(), 5); // grew by user + assistant
}

#[test]
fn policy_denies_a_tool() {
    use forge::agent::mock::ScriptTurn;
    use forge::permission::Policy;
    let agent = Agent::new(
        Box::new(MockProvider::new(vec![
            ScriptTurn::Tools(vec![call("c1", "bash", json!({"command": "echo hi"}))]),
            ScriptTurn::Answer("done".into()),
        ])),
        Registry::builtin(),
        5,
    )
    .with_policy(Policy::new().deny("bash"));
    let outcome = agent.run("go").unwrap();
    assert_eq!(outcome.tool_calls, 1);
    // The denied tool must not have run; the model still got a tool result.
    assert_eq!(outcome.final_text, "done");
}

#[test]
fn prompt_tool_denied_without_approver() {
    use forge::agent::mock::ScriptTurn;
    let agent = Agent::new(
        Box::new(MockProvider::new(vec![
            ScriptTurn::Tools(vec![call(
                "c1",
                "write_file",
                json!({"path": "x", "content": "y"}),
            )]),
            ScriptTurn::Answer("done".into()),
        ])),
        Registry::builtin(),
        5,
    );
    let outcome = agent.run("go").unwrap();
    assert_eq!(outcome.tool_calls, 1);
    assert_eq!(outcome.final_text, "done");
}

#[test]
fn approver_allows_a_prompt_tool() {
    use forge::agent::mock::ScriptTurn;
    let dir = tempfile::tempdir().unwrap();
    let agent = Agent::new(
        Box::new(MockProvider::new(vec![
            ScriptTurn::Tools(vec![call(
                "c1",
                "write_file",
                json!({"path": "x.txt", "content": "hi"}),
            )]),
            ScriptTurn::Answer("done".into()),
        ])),
        Registry::builtin(),
        5,
    )
    .with_workspace(dir.path())
    .with_approver(Box::new(|_| true));
    let outcome = agent.run("go").unwrap();
    assert_eq!(outcome.tool_calls, 1);
    assert!(dir.path().join("x.txt").exists());
}

#[test]
fn git_status_reports_changes() {
    let dir = tempfile::tempdir().unwrap();
    // Initialize a git repo.
    let init = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(init.status.success());
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let ctx = ToolContext::new(dir.path().to_path_buf());
    let registry = Registry::builtin();
    let status = registry.get("git_status").unwrap();
    let res = status.run(&json!({}), &ctx).unwrap();
    assert!(res.ok);
    assert!(res.output.contains("a.txt"));
}
