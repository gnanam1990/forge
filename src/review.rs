//! Code / PR review: a heuristic reviewer that inspects a diff and flags common
//! issues. A real model-based reviewer can replace the heuristics later.

/// A single review finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// The result of a review.
#[derive(Debug, Clone, Default)]
pub struct Review {
    pub findings: Vec<Finding>,
}

impl Review {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Review a diff (unified format) for common issues.
pub fn review_diff(diff: &str) -> Review {
    let mut review = Review::default();
    let mut added_lines = 0usize;
    let mut removed_lines = 0usize;

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            added_lines += 1;
            let content = &line[1..];
            let lower = content.to_lowercase();
            if lower.contains("todo") || lower.contains("fixme") {
                review.findings.push(Finding {
                    severity: Severity::Warning,
                    message: format!("TODO/FIXME left in code: {content}"),
                });
            }
            if lower.contains("console.log") || lower.contains("println!") {
                review.findings.push(Finding {
                    severity: Severity::Info,
                    message: format!("debug print left in code: {content}"),
                });
            }
            if lower.contains("password") || lower.contains("secret") || lower.contains("api_key") {
                review.findings.push(Finding {
                    severity: Severity::Error,
                    message: format!("possible secret in code: {content}"),
                });
            }
        } else if line.starts_with('-') && !line.starts_with("---") {
            removed_lines += 1;
        }
    }

    if added_lines > 200 {
        review.findings.push(Finding {
            severity: Severity::Warning,
            message: format!("large diff: {added_lines} lines added — consider splitting"),
        });
    }
    if added_lines == 0 && removed_lines == 0 {
        review.findings.push(Finding {
            severity: Severity::Info,
            message: "no changes detected in the diff".into(),
        });
    }

    review
}

/// Review a diff with a model, falling back to the heuristic reviewer if the
/// model call fails. The model is asked to return findings as JSON.
pub fn review_with_provider(diff: &str, provider: &dyn crate::agent::Provider) -> Review {
    let prompt = format!(
        "Review the following code diff and return a JSON array of findings. \
         Each finding is an object with \"severity\" (one of info, warning, error) \
         and \"message\". Return only the JSON array.\n\n{diff}"
    );
    let messages = vec![
        crate::agent::Message::System("You are a code reviewer.".into()),
        crate::agent::Message::User(prompt),
    ];
    match provider.complete(&messages) {
        Ok(reply) => parse_findings(&reply.content).unwrap_or_else(|| review_diff(diff)),
        Err(_) => review_diff(diff),
    }
}

/// Parse a JSON array of findings from a model reply.
fn parse_findings(text: &str) -> Option<Review> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    let slice = &text[start..=end];
    let value: serde_json::Value = serde_json::from_str(slice).ok()?;
    let array = value.as_array()?;
    let mut review = Review::default();
    for item in array {
        let severity = match item.get("severity").and_then(serde_json::Value::as_str) {
            Some("error") => Severity::Error,
            Some("warning") => Severity::Warning,
            _ => Severity::Info,
        };
        let message = item
            .get("message")
            .and_then(serde_json::Value::as_str)?
            .to_string();
        review.findings.push(Finding { severity, message });
    }
    Some(review)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_todo_and_secret() {
        let diff = "+fn main() {\n+    // TODO: fix this\n+    let api_key = \"sk-123\";\n+}\n";
        let review = review_diff(diff);
        assert!(review.findings.iter().any(|f| f.message.contains("TODO")));
        assert!(review
            .findings
            .iter()
            .any(|f| f.severity == Severity::Error));
    }

    #[test]
    fn clean_diff_has_no_findings() {
        let diff = "+fn add(a: i32, b: i32) -> i32 { a + b }\n";
        let review = review_diff(diff);
        assert!(review.is_clean());
    }

    #[test]
    fn empty_diff_is_info() {
        let review = review_diff("");
        assert!(review
            .findings
            .iter()
            .any(|f| f.message.contains("no changes")));
    }

    #[test]
    fn parses_model_findings() {
        let text = r#"[{"severity":"error","message":"secret leaked"},{"severity":"warning","message":"todo left"}]"#;
        let review = parse_findings(text).unwrap();
        assert_eq!(review.findings.len(), 2);
        assert_eq!(review.findings[0].severity, Severity::Error);
        assert_eq!(review.findings[1].severity, Severity::Warning);
    }

    #[test]
    fn parse_findings_falls_back_on_bad_json() {
        assert!(parse_findings("not json").is_none());
    }
}
