use std::collections::HashMap;
use std::sync::OnceLock;

use hashbrown::HashSet;
use regex::Regex;
use serde::Deserialize;

use crate::core::text::fold_accents;

const KNOWN_SKILLS: &[&str] = &[
    // Languages
    "rust",
    "python",
    "typescript",
    "javascript",
    "java",
    "c++",
    "c#",
    "c",
    "go",
    "php",
    "ruby",
    "swift",
    "kotlin",
    "dart",
    "scala",
    "elixir",
    "haskell",
    "r",
    "matlab",
    "perl",
    "lua",
    "zig",
    // Web frameworks
    "react",
    "angular",
    "vue",
    "next.js",
    "nuxt",
    "svelte",
    "nestjs",
    "express",
    "fastapi",
    "django",
    "flask",
    "spring",
    "rails",
    "laravel",
    "asp.net",
    ".net",
    "blazor",
    "htmx",
    // Databases
    "sql",
    "postgresql",
    "mysql",
    "sqlite",
    "mariadb",
    "mongodb",
    "elasticsearch",
    "cassandra",
    "dynamodb",
    "redis",
    "neo4j",
    "cockroachdb",
    "supabase",
    "firebase",
    // Cloud
    "aws",
    "azure",
    "gcp",
    "cloudflare",
    "vercel",
    "heroku",
    "digitalocean",
    "s3",
    "ec2",
    "lambda",
    "ecs",
    "gke",
    "aks",
    "eks",
    // DevOps / infra
    "docker",
    "kubernetes",
    "terraform",
    "ansible",
    "helm",
    "argocd",
    "gitlab ci",
    "github actions",
    "circleci",
    "jenkins",
    "prometheus",
    "grafana",
    "datadog",
    "nginx",
    "linux",
    "ci/cd",
    // Data / ML
    "machine learning",
    "deep learning",
    "nlp",
    "llm",
    "pytorch",
    "tensorflow",
    "keras",
    "scikit-learn",
    "pandas",
    "numpy",
    "spark",
    "kafka",
    "airflow",
    "dbt",
    "looker",
    "tableau",
    "power bi",
    // Rust ecosystem
    "tokio",
    "reqwest",
    "serde",
    "axum",
    "actix",
    "wasm",
    "clap",
    // Architecture / protocols
    "rest",
    "graphql",
    "grpc",
    "websockets",
    "oauth",
    "jwt",
    "openapi",
    "microservices",
    "event-driven",
    "cqrs",
    "ddd",
    // Practices / soft
    "agile",
    "scrum",
    "kanban",
    "tdd",
    "bdd",
    "devex",
    "sre",
    // Tools
    "git",
    "github",
    "gitlab",
    "jira",
    "figma",
    "postman",
];

/// Skills whose surface form is a common everyday word or single letter, so a
/// bare lowercase mention is too weak to count (RFC §5.4). They only count when
/// the original text uses a significant case (`Go`, `R`, `C`) or a strong
/// n-gram context (`go developer`, `spring boot`, …).
const AMBIGUOUS_SKILLS: &[&str] = &["go", "r", "c", "spring", "swift", "dart"];

/// Absolute-negation triggers only (RFC §5.4, restricted set). Stored
/// accent-folded and lowercase. A bare `pas` / `no` / `not` is intentionally
/// excluded, as are `pas seulement` / `not only` / `no longer`.
const NEGATION_TRIGGERS: &[&str] = &[
    "sans",
    "without",
    "pas de",
    "no experience",
    "aucune experience",
    "not familiar with",
];

/// How many characters before a skill mention are scanned for a negation
/// trigger. Wide enough for `no experience with X`, narrow enough to stay in the
/// same clause.
const NEGATION_WINDOW: usize = 40;

const ALIASES_TOML: &str = include_str!("../../assets/skills_aliases.toml");

/// Environment variable pointing to an optional external alias file whose
/// entries override the embedded table.
const ALIASES_FILE_ENV: &str = "HIRELENS_ALIASES_FILE";

#[derive(Debug, Deserialize, Default)]
struct AliasFile {
    #[serde(default)]
    aliases: HashMap<String, String>,
}

fn alias_map() -> &'static HashMap<String, String> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = parse_aliases(ALIASES_TOML);
        if let Some(path) = std::env::var_os(ALIASES_FILE_ENV) {
            if let Ok(contents) = std::fs::read_to_string(path) {
                // External entries override embedded ones.
                map.extend(parse_aliases(&contents));
            }
        }
        map
    })
}

fn parse_aliases(toml_src: &str) -> HashMap<String, String> {
    toml::from_str::<AliasFile>(toml_src)
        .map(|file| file.aliases)
        .unwrap_or_default()
        .into_iter()
        .map(|(variant, canonical)| (normalize_skill(&variant), normalize_skill(&canonical)))
        .filter(|(variant, canonical)| !variant.is_empty() && !canonical.is_empty())
        .collect()
}

/// Each known surface form paired with its compiled boundary regex and the
/// canonical skill it resolves to. Built from `KNOWN_SKILLS` (identity) plus the
/// alias table (variant → canonical).
struct SkillPattern {
    regex: Regex,
    surface: String,
    canonical: String,
}

fn skill_patterns() -> &'static [SkillPattern] {
    static PATTERNS: OnceLock<Vec<SkillPattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let mut patterns: Vec<SkillPattern> = KNOWN_SKILLS
            .iter()
            .map(|skill| build_pattern(skill, skill))
            .collect();
        for (variant, canonical) in alias_map() {
            patterns.push(build_pattern(variant, canonical));
        }
        patterns
    })
}

fn build_pattern(surface: &str, canonical: &str) -> SkillPattern {
    // Boundary class mirrors `normalize_skill`: `+ # . /` are part of a skill
    // token (so `c++`, `c#`, `.net`, `ci/cd` match), everything else delimits.
    let pattern = format!(
        r"(?i)(^|[^a-z0-9+#./]){}($|[^a-z0-9+#./])",
        regex::escape(surface)
    );
    SkillPattern {
        regex: Regex::new(&pattern).expect("invalid skill pattern"),
        surface: surface.to_owned(),
        canonical: canonical.to_owned(),
    }
}

/// Case-sensitive whole-word patterns for ambiguous skills, used to detect a
/// significant-case mention (`Go`, `R`, `C`) in the original text.
fn ambiguous_cased_patterns() -> &'static HashMap<&'static str, Regex> {
    static MAP: OnceLock<HashMap<&'static str, Regex>> = OnceLock::new();
    MAP.get_or_init(|| {
        AMBIGUOUS_SKILLS
            .iter()
            .map(|surface| {
                let pattern = format!(
                    r"(?i)(^|[^A-Za-z0-9+#./])({})($|[^A-Za-z0-9+#./])",
                    regex::escape(surface)
                );
                (
                    *surface,
                    Regex::new(&pattern).expect("invalid cased pattern"),
                )
            })
            .collect()
    })
}

pub fn normalize_skill(skill: &str) -> String {
    skill
        .trim()
        .trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '+' && c != '#' && c != '/' && c != '.'
        })
        .to_lowercase()
}

/// Single-word tokens appearing in any *canonical* known skill. Used by keyword
/// extraction to keep the keyword signal orthogonal to the skill signal (RFC
/// §5.5 — `keyword_cov` is "hors skills dico").
///
/// Built from `KNOWN_SKILLS` only, never from alias keys: alias surface forms
/// include descriptive phrases (`google cloud platform`, `amazon web services`)
/// whose generic words (`platform`, `cloud`, `web`) are legitimate keywords.
pub(crate) fn skill_words() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = HashSet::new();
        for skill in KNOWN_SKILLS {
            set.extend(crate::core::text::tokenize_words(skill));
        }
        set
    })
}

pub fn skill_set(skills: &[String]) -> HashSet<String> {
    // NOTE: normalise only — never alias-canonicalise here. `skill_set` feeds the
    // anti-hallucination whitelist in `validation.rs`; collapsing aliases would
    // reject a skill the CV explicitly allowed (e.g. CV `k8s`, output `k8s`, but
    // a canonicalised whitelist would only hold `kubernetes`). Alias-aware
    // *matching* lives on the audit path, not the validation path.
    skills
        .iter()
        .map(|skill| normalize_skill(skill))
        .filter(|skill| !skill.is_empty())
        .collect()
}

pub fn extract_local_skills(text: &str) -> HashSet<String> {
    let lower = text.to_lowercase();
    let mut found = HashSet::new();
    for pattern in skill_patterns() {
        for matched in pattern.regex.find_iter(&lower) {
            if is_negated(&lower, matched.start()) {
                continue;
            }
            if AMBIGUOUS_SKILLS.contains(&pattern.surface.as_str())
                && !ambiguous_context_present(&pattern.surface, text, &lower)
            {
                continue;
            }
            found.insert(pattern.canonical.clone());
            break;
        }
    }
    found
}

/// Is the skill mention immediately preceded by an absolute-negation trigger?
fn is_negated(lower: &str, match_start: usize) -> bool {
    let window = preceding_window(lower, match_start, NEGATION_WINDOW);
    let folded = fold_accents(window);
    NEGATION_TRIGGERS
        .iter()
        .any(|trigger| folded.contains(trigger))
}

/// Up to `max` characters of `text` ending at byte index `end`, clipped to a
/// char boundary so multibyte input never panics.
fn preceding_window(text: &str, end: usize, max: usize) -> &str {
    let prefix = &text[..end];
    let start = prefix
        .char_indices()
        .rev()
        .take(max)
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0);
    &prefix[start..]
}

/// Does an ambiguous skill have enough signal to count — a significant-case
/// mention in the original text, or a strong n-gram context?
fn ambiguous_context_present(surface: &str, original: &str, lower: &str) -> bool {
    if has_cased_mention(surface, original) {
        return true;
    }
    let folded = fold_accents(lower);
    ambiguous_contexts(surface)
        .iter()
        .any(|phrase| folded.contains(phrase))
}

fn has_cased_mention(surface: &str, original: &str) -> bool {
    let Some(regex) = ambiguous_cased_patterns().get(surface) else {
        return false;
    };
    regex
        .captures_iter(original)
        .filter_map(|caps| caps.get(2))
        .any(|token| token.as_str().chars().any(|c| c.is_uppercase()))
}

/// Strong disambiguating n-grams per ambiguous skill (lowercase, accent-folded).
fn ambiguous_contexts(surface: &str) -> &'static [&'static str] {
    match surface {
        "go" => &[
            "go developer",
            "go programming",
            "go language",
            "langage go",
            "go lang",
            "go routine",
            "goroutine",
            "go module",
        ],
        "r" => &[
            "r language",
            "langage r",
            "r programming",
            "r studio",
            "rstudio",
            "statistical r",
        ],
        "c" => &[
            "c language",
            "langage c",
            "c programming",
            "programmation c",
            "embedded c",
            "c developer",
        ],
        "spring" => &[
            "spring boot",
            "springboot",
            "spring framework",
            "spring mvc",
            "spring cloud",
            "spring security",
            "framework spring",
        ],
        "swift" => &[
            "swift developer",
            "swiftui",
            "swift programming",
            "ios swift",
            "langage swift",
        ],
        "dart" => &[
            "dart language",
            "langage dart",
            "dart programming",
            "flutter dart",
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(text: &str) -> HashSet<String> {
        extract_local_skills(text)
    }

    #[test]
    fn extracts_alias_surface_forms() {
        let found = extract("Strong with k8s and golang in production.");
        assert!(found.contains("kubernetes"));
        assert!(
            found.contains("go"),
            "golang is an unambiguous alias for go"
        );
    }

    #[test]
    fn ambiguous_go_needs_case_or_context() {
        // bare lowercase mention → rejected
        assert!(!extract("I will go to the office.").contains("go"));
        // strong n-gram context → accepted
        assert!(extract("Backend in go programming since 2019.").contains("go"));
        // significant case → accepted
        assert!(extract("Worked as a Go engineer.").contains("go"));
    }

    #[test]
    fn ambiguous_single_letters_need_case() {
        assert!(!extract("grade c in school, vitamin c daily").contains("c"));
        let cased = extract("Wrote drivers in C and ran analysis in R daily.");
        assert!(cased.contains("c"));
        assert!(cased.contains("r"));
    }

    #[test]
    fn ambiguous_spring_needs_context() {
        assert!(!extract("Hired in the spring of 2021.").contains("spring"));
        assert!(extract("Built REST APIs with spring boot.").contains("spring"));
    }

    #[test]
    fn absolute_negation_cancels_skill() {
        assert!(!extract("Candidate has no experience with Java.").contains("java"));
        assert!(!extract("Poste sans Python ni Django.").contains("python"));
        assert!(!extract("Not familiar with Kubernetes yet.").contains("kubernetes"));
        assert!(!extract("Aucune expérience en Rust.").contains("rust"));
    }

    #[test]
    fn negation_does_not_overreach() {
        // bare "pas" must not cancel
        assert!(extract("Je n'ai pas peur de coder en C++ au quotidien.").contains("c++"));
        // "not only" is not an absolute trigger
        assert!(extract("We use not only Java but also Kotlin.").contains("java"));
        // plain positive mention stays
        assert!(extract("Five years of Python experience.").contains("python"));
    }
}
