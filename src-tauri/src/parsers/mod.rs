pub mod claude_code;
pub mod claude_desktop;
pub mod cursor;
pub mod openclaw;
pub mod pricing;

use crate::models::{Action, Agent, AgentType};
use anyhow::Result;

/// Each agent parser tracks its own read position and emits new Actions on poll.
pub trait AgentParser: Send + Sync {
    fn agent_id(&self) -> &str;
    fn parse_new_actions(&mut self) -> Result<Vec<Action>>;
    /// Reset file positions so the parser re-reads from the beginning.
    fn reset_position(&mut self);
}

/// Orchestrator that holds all active parsers and merges their output.
pub struct AgentWatcher {
    parsers: Vec<Box<dyn AgentParser>>,
}

impl AgentWatcher {
    /// Create an AgentWatcher with parsers for each discovered agent.
    pub fn new(agents: &[Agent]) -> Self {
        let mut parsers: Vec<Box<dyn AgentParser>> = Vec::new();

        for agent in agents {
            match agent.agent_type {
                AgentType::ClaudeCode => {
                    if let Some(projects_dir) = agent
                        .metadata
                        .get("projects_dir")
                        .and_then(|v| v.as_str())
                    {
                        parsers.push(Box::new(claude_code::ClaudeCodeParser::new(
                            projects_dir.into(),
                        )));
                    }
                }
                AgentType::ClaudeDesktop => {
                    parsers.push(Box::new(claude_desktop::ClaudeDesktopParser::new()));
                }
                AgentType::Cursor => {
                    parsers.push(Box::new(cursor::CursorParser::new()));
                }
                AgentType::OpenClaw => {
                    parsers.push(Box::new(openclaw::OpenClawParser::new()));
                }
                _ => {}
            }
        }

        Self { parsers }
    }

    /// Reset all parsers so they re-read from the beginning of log files.
    pub fn reset(&mut self) {
        for parser in &mut self.parsers {
            parser.reset_position();
        }
    }

    /// Poll all parsers and return merged, time-sorted actions.
    pub fn poll(&mut self) -> Result<Vec<Action>> {
        let mut all_actions = Vec::new();

        for parser in &mut self.parsers {
            match parser.parse_new_actions() {
                Ok(actions) => all_actions.extend(actions),
                Err(e) => {
                    eprintln!("[AgentWatcher] Error polling {}: {}", parser.agent_id(), e);
                }
            }
        }

        all_actions.sort_by_key(|a| a.timestamp);
        Ok(all_actions)
    }
}
