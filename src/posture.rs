//! The posture: a session-scoped effort level that raises the turn budget and
//! reasoning effort, and unlocks orchestration. Mirrors the zeromaxing PRD.

use serde::{Deserialize, Serialize};

/// The effort levels forge supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effort {
    Auto,
    Balanced,
    Thorough,
    Zeromaxing,
}

impl Effort {
    /// Parse an effort name, case-insensitively.
    pub fn parse(name: &str) -> Option<Effort> {
        match name.to_lowercase().as_str() {
            "auto" => Some(Effort::Auto),
            "balanced" => Some(Effort::Balanced),
            "thorough" => Some(Effort::Thorough),
            "zeromaxing" => Some(Effort::Zeromaxing),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Effort::Auto => "auto",
            Effort::Balanced => "balanced",
            Effort::Thorough => "thorough",
            Effort::Zeromaxing => "zeromaxing",
        }
    }
}

/// The resolved posture for a session.
#[derive(Debug, Clone)]
pub struct Posture {
    pub effort: Effort,
    pub turn_budget: usize,
    pub reasoning_effort: Option<String>,
    pub self_correct: bool,
    pub orchestrate: bool,
}

impl Posture {
    /// Resolve the posture for an effort level.
    pub fn from_effort(effort: Effort) -> Self {
        match effort {
            Effort::Auto => Posture {
                effort,
                turn_budget: 80,
                reasoning_effort: None,
                self_correct: false,
                orchestrate: false,
            },
            Effort::Balanced => Posture {
                effort,
                turn_budget: 160,
                reasoning_effort: Some("medium".into()),
                self_correct: false,
                orchestrate: false,
            },
            Effort::Thorough => Posture {
                effort,
                turn_budget: 160,
                reasoning_effort: Some("high".into()),
                self_correct: true,
                orchestrate: false,
            },
            Effort::Zeromaxing => Posture {
                effort,
                turn_budget: 320,
                reasoning_effort: Some("high".into()),
                self_correct: true,
                orchestrate: true,
            },
        }
    }

    /// An honest description of what this posture changes relative to `other`.
    pub fn delta(&self, other: &Posture) -> String {
        let mut changes = Vec::new();
        if self.turn_budget != other.turn_budget {
            changes.push(format!(
                "turn budget: {} -> {}",
                other.turn_budget, self.turn_budget
            ));
        }
        if self.reasoning_effort != other.reasoning_effort {
            changes.push(format!(
                "reasoning effort: {} -> {}",
                other.reasoning_effort.as_deref().unwrap_or("unset"),
                self.reasoning_effort.as_deref().unwrap_or("unset")
            ));
        }
        if self.self_correct != other.self_correct {
            changes.push(format!(
                "self-correct: {} -> {}",
                other.self_correct, self.self_correct
            ));
        }
        if self.orchestrate != other.orchestrate {
            changes.push(format!(
                "orchestrate tool: {} -> {}",
                if other.orchestrate {
                    "offered"
                } else {
                    "absent"
                },
                if self.orchestrate {
                    "offered"
                } else {
                    "absent"
                }
            ));
        }
        if changes.is_empty() {
            "no changes".to_string()
        } else {
            changes.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_effort() {
        assert_eq!(Effort::parse("zeromaxing"), Some(Effort::Zeromaxing));
        assert_eq!(Effort::parse("AUTO"), Some(Effort::Auto));
        assert_eq!(Effort::parse("bogus"), None);
    }

    #[test]
    fn zeromaxing_raises_budget_and_unlocks_orchestrate() {
        let posture = Posture::from_effort(Effort::Zeromaxing);
        assert_eq!(posture.turn_budget, 320);
        assert!(posture.orchestrate);
        assert!(posture.self_correct);
    }

    #[test]
    fn delta_is_honest() {
        let auto = Posture::from_effort(Effort::Auto);
        let max = Posture::from_effort(Effort::Zeromaxing);
        let delta = max.delta(&auto);
        assert!(delta.contains("turn budget: 80 -> 320"));
        assert!(delta.contains("orchestrate tool: absent -> offered"));
    }
}
