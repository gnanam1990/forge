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
}
