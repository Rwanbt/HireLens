//! Text normalisation helpers shared by the offline matching engine.
//!
//! Kept deliberately small for now: the robust character state-machine tokenizer
//! (handling `c++`, `c#`, `node.js`, `ci/cd`) and n-grams arrive with the lexical
//! similarity work (RFC §5.2, P2). What lives here is only what the P1 skill
//! extraction and keyword coverage need.

use std::sync::OnceLock;

use hashbrown::HashSet;

/// Vetted FR+EN stopword list (RFC §5.2). It is intentionally NOT a blind union:
/// tech false-friends that are stopwords in one language but carriers in the
/// other or in code (`main`, `car`, `pour`, `go`, `son`) are deliberately left
/// out so the lexical recall of keyword extraction is not sabotaged.
const STOPWORDS: &[&str] = &[
    // English
    "the", "and", "for", "are", "but", "not", "you", "all", "any", "can", "had", "has", "have",
    "her", "his", "our", "out", "their", "they", "this", "that", "these", "those", "was", "were",
    "will", "with", "would", "should", "could", "from", "into", "over", "under", "your", "who",
    "what", "which", "when", "where", "how", "than", "too", "very", "just", "also", "about",
    "after", "before", "during", "per", "such", "some", "more", "most", "been", "being", "does",
    "did", "yes", "its", "let", "may", "might", "must", "off", "own", "via", // French
    "les", "des", "une", "aux", "dans", "sur", "sous", "par", "avec", "sans", "ses", "leur",
    "leurs", "cette", "cet", "ces", "qui", "que", "quoi", "dont", "est", "sont", "ete", "etre",
    "avoir", "nous", "vous", "ils", "elles", "plus", "moins", "tres", "aussi", "ainsi", "donc",
    "mais", "comme", "chez", "entre", "vers", "lors", "afin", "tout", "tous", "toute", "toutes",
    "notre", "votre", "leurs", "elle", "ont", "vos", "nos",
];

/// Lowercased, accent-folded set of stopwords, built once.
fn stopword_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| STOPWORDS.iter().copied().collect())
}

/// Is `token` a stopword? `token` is expected lowercased + accent-folded.
pub fn is_stopword(token: &str) -> bool {
    stopword_set().contains(token)
}

/// Split text into lowercased, accent-folded alphanumeric word tokens.
///
/// This is the deliberately simple keyword tokenizer (métier / soft words). It
/// splits on every non-alphanumeric character, so it does NOT preserve symbol
/// skills like `c++` — that is the job of the P2 state-machine tokenizer. Use it
/// for keyword coverage, not for skill detection.
pub fn tokenize_words(text: &str) -> Vec<String> {
    fold_accents(&text.to_lowercase())
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_owned())
        .collect()
}

/// State-machine tokenizer that preserves symbol-bearing skill tokens
/// (`c++`, `c#`, `f#`, `node.js`, `ci/cd`, `.net`) intact (RFC §5.2). Folds
/// accents + lowercases. Rules, by character:
/// - alphanumeric → part of the current token;
/// - `+` / `#` after token content → trailing symbol kept (`c++`, `c#`);
/// - `.` / `/` kept only when it sits *between* alphanumerics, or as a leading
///   `.` before one (`.net`) — a trailing `.` (sentence end) is a separator;
/// - anything else → emits the current token.
///
/// Used for lexical similarity, where symbol skills must survive as one token —
/// unlike `tokenize_words`, which is the coarse keyword tokenizer.
pub fn tokenize(text: &str) -> Vec<String> {
    let chars: Vec<char> = fold_accents(&text.to_lowercase()).chars().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();
    for (index, &c) in chars.iter().enumerate() {
        let next_is_alnum = chars
            .get(index + 1)
            .map(|next| next.is_alphanumeric())
            .unwrap_or(false);
        // A char belongs to the current token if it is alphanumeric, a trailing
        // `+`/`#` (c++, c#), or a `.`/`/` sitting before another alphanumeric
        // (node.js, ci/cd, leading .net). Anything else closes the token.
        let part_of_token = c.is_alphanumeric()
            || (matches!(c, '+' | '#') && !current.is_empty())
            || (matches!(c, '.' | '/') && next_is_alnum);
        if part_of_token {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

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

    #[test]
    fn tokenizes_into_folded_lowercase_words() {
        let tokens = tokenize_words("Built a Payments-Platform, déployé en CI/CD.");
        assert_eq!(
            tokens,
            vec!["built", "a", "payments", "platform", "deploye", "en", "ci", "cd"]
        );
    }

    #[test]
    fn stopwords_cover_both_languages_but_spare_false_friends() {
        assert!(is_stopword("the"));
        assert!(is_stopword("avec"));
        // tech false-friends must NOT be stopwords
        assert!(!is_stopword("main"));
        assert!(!is_stopword("car"));
        assert!(!is_stopword("pour"));
        assert!(!is_stopword("go"));
    }

    #[test]
    fn tokenize_preserves_symbol_skills() {
        let tokens = tokenize("Built with C++, C#, Node.js and CI/CD pipelines.");
        assert_eq!(
            tokens,
            vec![
                "built",
                "with",
                "c++",
                "c#",
                "node.js",
                "and",
                "ci/cd",
                "pipelines"
            ]
        );
    }

    #[test]
    fn tokenize_handles_leading_dot_and_trailing_period() {
        // leading dot kept (.net), trailing sentence period dropped
        assert_eq!(tokenize("Uses .NET daily."), vec!["uses", ".net", "daily"]);
        // a trailing symbol period must not stick to the token
        assert_eq!(tokenize("end."), vec!["end"]);
    }
}
