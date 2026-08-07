//! An interactive terminal UI: a line-based REPL that runs the agent turn by
//! turn, prompts for approval on mutating tools, and persists the session.

use std::io::Write;

use crate::agent::{http::HttpProvider, Agent};
use crate::config::Config;
use crate::error::Result;
use crate::session::{default_sessions_dir, Session};
use crate::tools::Registry;

/// Run the interactive chat loop.
pub fn run_chat(config: &Config) -> Result<()> {
    let provider = HttpProvider::new(&config.provider)?;
    let turns = config.max_turns.unwrap_or(10);
    let dir = default_sessions_dir()?;
    let id = format!(
        "sess-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let mut session = Session::new(&id);

    let agent = Agent::new(Box::new(provider), Registry::builtin(), turns).with_approver(Box::new(
        |tool| {
            print!("allow {tool}? [y/N] ");
            let _ = std::io::stdout().flush();
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            line.trim().eq_ignore_ascii_case("y")
        },
    ));

    println!("forge chat — type a prompt, or /help for commands.");
    loop {
        print!("forge> ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        match line {
            "/exit" | "/quit" => break,
            "/help" => print_help(),
            "/sessions" => {
                let ids = Session::list(&dir)?;
                if ids.is_empty() {
                    println!("no saved sessions");
                } else {
                    for id in ids {
                        println!("{id}");
                    }
                }
            }
            "/resume" => {
                println!("usage: /resume <session-id>");
            }
            _ if line.starts_with("/resume ") => {
                let id = line["/resume ".len()..].trim();
                session = Session::load(&dir, id)?;
                println!("resumed session {id} ({} messages)", session.messages.len());
            }
            "" => {}
            prompt => match agent.run_into(&mut session.messages, prompt) {
                Ok(outcome) => {
                    println!("{}", outcome.final_text);
                    session.save(&dir)?;
                }
                Err(e) => eprintln!("error: {e}"),
            },
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        "commands:\n  /help        show this help\n  /sessions    list saved sessions\n  \
         /resume <id> resume a session\n  /exit        quit"
    );
}
