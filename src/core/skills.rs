use std::sync::OnceLock;

use hashbrown::HashSet;
use regex::Regex;

const KNOWN_SKILLS: &[&str] = &[
    // Languages
    "rust",
    "python",
    "typescript",
    "javascript",
    "java",
    "c++",
    "c#",
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

static SKILL_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

fn skill_patterns() -> &'static [(Regex, &'static str)] {
    SKILL_PATTERNS.get_or_init(|| {
        KNOWN_SKILLS
            .iter()
            .map(|skill| {
                let pattern = format!(
                    r"(?i)(^|[^a-z0-9+#./]){}($|[^a-z0-9+#./])",
                    regex::escape(skill)
                );
                (Regex::new(&pattern).expect("invalid skill pattern"), *skill)
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

pub fn skill_set(skills: &[String]) -> HashSet<String> {
    skills
        .iter()
        .map(|skill| normalize_skill(skill))
        .filter(|skill| !skill.is_empty())
        .collect()
}

pub fn extract_local_skills(text: &str) -> HashSet<String> {
    let haystack = text.to_lowercase();
    skill_patterns()
        .iter()
        .filter(|(re, _)| re.is_match(&haystack))
        .map(|(_, skill)| normalize_skill(skill))
        .collect()
}
