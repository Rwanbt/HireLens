use hashbrown::HashSet;
use regex::Regex;

const KNOWN_SKILLS: &[&str] = &[
    "rust",
    "python",
    "typescript",
    "javascript",
    "java",
    "c++",
    "go",
    "sql",
    "postgresql",
    "mysql",
    "redis",
    "aws",
    "azure",
    "gcp",
    "docker",
    "kubernetes",
    "terraform",
    "linux",
    "react",
    "node.js",
    "tokio",
    "reqwest",
    "serde",
    "machine learning",
    "nlp",
    "llm",
    "ci/cd",
    "github actions",
    "microservices",
    "rest",
    "graphql",
];

pub fn normalize_skill(skill: &str) -> String {
    skill
        .trim()
        .trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '+' && c != '#' && c != '/' && c != '.'
        })
        .to_lowercase()
}

pub fn skill_set(skills: &[String]) -> HashSet<String> {
    skills
        .iter()
        .map(|skill| normalize_skill(skill))
        .filter(|skill| !skill.is_empty())
        .collect()
}

pub fn extract_local_skills(text: &str) -> HashSet<String> {
    let haystack = text.to_lowercase();
    KNOWN_SKILLS
        .iter()
        .filter(|skill| {
            let pattern = format!(
                r"(?i)(^|[^a-z0-9+#./]){}($|[^a-z0-9+#./])",
                regex::escape(skill)
            );
            Regex::new(&pattern)
                .map(|re| re.is_match(&haystack))
                .unwrap_or(false)
        })
        .map(|skill| normalize_skill(skill))
        .collect()
}
