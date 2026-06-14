//! Text normalisation helpers shared by the offline matching engine.
//!
//! Kept deliberately small for now: the robust character state-machine tokenizer
//! (handling `c++`, `c#`, `node.js`, `ci/cd`) and n-grams arrive with the lexical
//! similarity work (RFC §5.2, P2). What lives here is only what the P1 skill
//! extraction and keyword coverage need.

/// Replace common French accented letters with their unaccented ASCII form.
///
/// Folding is for *matching only* (negation triggers, keyword comparison). The
/// original text is never mutated — verbatim output stays untouched. Folding is
/// done per character, so the result is independent of byte width.
pub fn fold_accents(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'î' | 'ï' | 'í' => 'i',
            'ò' | 'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_french_accents() {
        assert_eq!(fold_accents("aucune expérience"), "aucune experience");
        assert_eq!(fold_accents("déçu à Noël"), "decu a Noel");
    }

    #[test]
    fn leaves_ascii_untouched() {
        assert_eq!(
            fold_accents("no experience with java"),
            "no experience with java"
        );
    }
}
