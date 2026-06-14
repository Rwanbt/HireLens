//! Line-level diff between the original CV text and the optimized output, used
//! by the GUI to show what changed. Wraps `similar` so the algorithm lives in
//! one place.
//!
//! This is a *display* aid only. The anti-hallucination guarantee is enforced
//! upstream in [`crate::core::validation::validate_adaptation`], not here — a
//! line being "Added" in this diff just means it is new text in the rendered
//! output (e.g. a section heading), not that it bypassed validation.

use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Unchanged,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}

/// Computes a line-by-line diff. Each `DiffLine::text` carries no trailing newline.
pub fn compute_diff(original: &str, adapted: &str) -> Vec<DiffLine> {
    TextDiff::from_lines(original, adapted)
        .iter_all_changes()
        .map(|change| {
            let kind = match change.tag() {
                ChangeTag::Equal => DiffKind::Unchanged,
                ChangeTag::Insert => DiffKind::Added,
                ChangeTag::Delete => DiffKind::Removed,
            };
            DiffLine {
                kind,
                text: change.value().trim_end_matches('\n').to_owned(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_input_is_all_unchanged() {
        let diff = compute_diff("a\nb\n", "a\nb\n");
        assert_eq!(diff.len(), 2);
        assert!(diff.iter().all(|line| line.kind == DiffKind::Unchanged));
    }

    #[test]
    fn detects_added_and_removed_lines() {
        let diff = compute_diff("keep\nold\n", "keep\nnew\n");
        let added: Vec<&str> = diff
            .iter()
            .filter(|line| line.kind == DiffKind::Added)
            .map(|line| line.text.as_str())
            .collect();
        let removed: Vec<&str> = diff
            .iter()
            .filter(|line| line.kind == DiffKind::Removed)
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(added, vec!["new"]);
        assert_eq!(removed, vec!["old"]);
        assert!(diff
            .iter()
            .any(|line| line.kind == DiffKind::Unchanged && line.text == "keep"));
    }

    #[test]
    fn text_carries_no_trailing_newline() {
        let diff = compute_diff("alpha\n", "alpha\n");
        assert_eq!(diff[0].text, "alpha");
    }
}
