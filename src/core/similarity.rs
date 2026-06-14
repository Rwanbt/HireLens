//! Lexical similarity between a job (query) and a CV (document) — RFC §5.3.
//!
//! Pure, corpus-free, deterministic. P2 starts with the robust first stage the
//! RFC prescribes: term-overlap with BM25 frequency saturation (no IDF table
//! yet). The BM25 + static-minimal-IDF escalation is deliberately deferred until
//! the golden set shows non-tech pairs are mis-scored (RFC §5.3, "implémentation
//! progressive").

use std::collections::HashMap;

use crate::core::text::{is_stopword, tokenize};

/// BM25 term-frequency saturation constant.
const K1: f32 = 1.2;
/// Tokens shorter than this are dropped before scoring (RFC §5.3). Short symbol
/// skills are already captured by the skill dimension, so this only trims noise.
const MIN_TOKEN_LEN: usize = 3;

/// Lexical similarity ∈ [0, 1]: how well the CV covers the job's vocabulary,
/// rewarding repeated coverage with diminishing returns (saturation).
///
/// `sum over unique job terms of sat(tf_in_cv) / number of unique job terms`,
/// where `sat(tf) = tf / (tf + K1)` (0 when absent, →1 as frequency grows).
pub fn lexical_similarity(job_text: &str, cv_text: &str) -> f32 {
    let query = significant_term_set(job_text);
    lexical_similarity_with_query(cv_text, &query)
}

/// Pre-compute the unique significant terms of a job text so the same query
/// can be scored against many CV bullets without re-tokenising the job each
/// time (RFC §0.2, perf guard-fou — ChatGPT/DeepSeek).
pub fn query_terms(job_text: &str) -> Vec<String> {
    significant_term_set(job_text)
}

/// Score `cv_text` against a pre-computed query (returned by [`query_terms`]).
/// Equivalent to `lexical_similarity(job_text, cv_text)` but avoids repeating
/// the job tokenisation when the same query is used many times.
pub fn lexical_similarity_with_query(cv_text: &str, query: &[String]) -> f32 {
    if query.is_empty() {
        return 0.0;
    }
    let document = term_frequencies(cv_text);
    if document.is_empty() {
        return 0.0;
    }

    let total: f32 = query
        .iter()
        .map(|term| match document.get(term) {
            Some(&tf) => saturate(tf),
            None => 0.0,
        })
        .sum();
    (total / query.len() as f32).clamp(0.0, 1.0)
}

/// `tf / (tf + K1)`: 0 at tf=0, rising with diminishing returns toward 1.
fn saturate(tf: usize) -> f32 {
    let tf = tf as f32;
    tf / (tf + K1)
}

/// Unique significant terms (deduplicated): drops stopwords and short tokens.
fn significant_term_set(text: &str) -> Vec<String> {
    let mut seen = HashMap::new();
    for term in tokenize(text) {
        if is_significant(&term) {
            seen.entry(term).or_insert(());
        }
    }
    seen.into_keys().collect()
}

/// Term → frequency map of the significant terms of `text`.
fn term_frequencies(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for term in tokenize(text) {
        if is_significant(&term) {
            *counts.entry(term).or_insert(0) += 1;
        }
    }
    counts
}

fn is_significant(term: &str) -> bool {
    term.chars().count() >= MIN_TOKEN_LEN && !is_stopword(term)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_overlap_is_zero() {
        assert_eq!(
            lexical_similarity("rust backend", "marketing copywriter"),
            0.0
        );
    }

    #[test]
    fn empty_inputs_are_zero() {
        assert_eq!(lexical_similarity("", "anything here"), 0.0);
        assert_eq!(lexical_similarity("something", ""), 0.0);
    }

    #[test]
    fn single_term_single_occurrence_matches_saturation() {
        // one query term, present once in the CV → sat(1) = 1/(1+1.2)
        let sim = lexical_similarity("payments", "payments dashboard");
        assert!((sim - 1.0 / 2.2).abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn higher_frequency_scores_higher() {
        let once = lexical_similarity("payments", "payments work");
        let thrice = lexical_similarity("payments", "payments payments payments work");
        assert!(thrice > once);
        assert!(thrice <= 1.0);
    }

    #[test]
    fn partial_coverage_is_proportional() {
        // two query terms, only one present once → (sat(1) + 0) / 2
        let sim = lexical_similarity("payments billing", "payments only");
        assert!((sim - (1.0 / 2.2) / 2.0).abs() < 1e-6, "got {sim}");
    }
}
