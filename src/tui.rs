//! A full-screen terminal UI built on ratatui + crossterm: a scrollable message
//! list, an input box, and a status bar. Submitting a prompt runs the agent and
//! appends the reply.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{Frame, Terminal};

use crate::agent::{http::HttpProvider, Agent};
use crate::config::Config;
use crate::error::Result;
use crate::session::{default_sessions_dir, Session};
use crate::tools::Registry;

/// The TUI application state.
struct App {
    messages: Vec<String>,
    input: String,
    status: String,
    agent: Agent,
    session: Session,
    sessions_dir: std::path::PathBuf,
}

impl App {
    fn new(agent: Agent, session: Session, sessions_dir: std::path::PathBuf) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            status: "ready".into(),
            agent,
            session,
            sessions_dir,
        }
    }

    /// Submit the current input as a prompt and run the agent.
    fn submit(&mut self) -> Result<()> {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() {
            return Ok(());
        }
        self.input.clear();
        self.messages.push(format!("> {prompt}"));
        self.status = "running…".into();
        match self.agent.run_into(&mut self.session.messages, &prompt) {
            Ok(outcome) => {
                self.messages.push(outcome.final_text);
                self.status = format!(
                    "{} turn(s), {} tool call(s)",
                    outcome.turns, outcome.tool_calls
                );
                let _ = self.session.save(&self.sessions_dir);
            }
            Err(e) => {
                self.messages.push(format!("error: {e}"));
                self.status = "error".into();
            }
        }
        Ok(())
    }
}

/// Run the full-screen TUI.
pub fn run_chat(config: &Config) -> Result<()> {
    let provider = HttpProvider::new(&config.provider)?;
    let turns = config.max_turns.unwrap_or(10);
    let sessions_dir = default_sessions_dir()?;
    let id = format!(
        "sess-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let session = Session::new(&id);
    let agent = Agent::new(Box::new(provider), Registry::builtin(), turns).with_approver(Box::new(
        |tool| {
            print!("allow {tool}? [y/N] ");
            let _ = io::stdout().flush();
            let mut line = String::new();
            let _ = io::stdin().read_line(&mut line);
            line.trim().eq_ignore_ascii_case("y")
        },
    ));
    let mut app = App::new(agent, session, sessions_dir);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Char(c)
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                                && c == 'c' =>
                        {
                            return Ok(());
                        }
                        KeyCode::Char(c) => app.input.push(c),
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Enter => app.submit()?,
                        _ => {}
                    }
                }
            }
        }
    }
}

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    let items: Vec<ListItem> = app
        .messages
        .iter()
        .map(|m| ListItem::new(m.as_str()))
        .collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("forge"));
    f.render_widget(list, chunks[0]);

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("input"));
    f.render_widget(input, chunks[1]);

    let status = Paragraph::new(app.status.as_str());
    f.render_widget(status, chunks[2]);
}
