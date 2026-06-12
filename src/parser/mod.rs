use std::path::Path;

use anyhow::{Context, Result};
use gray_matter::{engine::YAML, Matter};
use pulldown_cmark::{Event, Parser, Tag};
use serde::Deserialize;

use crate::core::{Cv, Education, Experience, JobDescription};

#[derive(Debug, Deserialize, Default)]
struct CvFrontmatter {
    name: Option<String>,
    headline: Option<String>,
    summary: Option<String>,
    skills: Option<Vec<String>>,
    experience: Option<Vec<Experience>>,
    education: Option<Vec<Education>>,
}

pub fn parse_cv_file(path: &Path) -> Result<Cv> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read CV {}", path.display()))?;
    parse_cv_markdown(&raw)
}

pub fn parse_cv_markdown(raw: &str) -> Result<Cv> {
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(raw);
    let data = parsed
        .data
        .map(|data| data.deserialize::<CvFrontmatter>())
        .transpose()?
        .unwrap_or_default();

    let mut cv = Cv {
        name: data.name,
        headline: data.headline,
        summary: data.summary,
        skills: data.skills.unwrap_or_default(),
        experience: data.experience.unwrap_or_default(),
        education: data.education.unwrap_or_default(),
        raw_markdown: raw.to_owned(),
    };

    if cv.skills.is_empty() {
        cv.skills = parse_skills_from_markdown(&parsed.content);
    }
    if cv.experience.is_empty() {
        cv.experience = parse_experience_from_markdown(&parsed.content);
    }

    Ok(cv)
}

pub fn parse_job_file(path: &Path) -> Result<JobDescription> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read job description {}", path.display()))?;
    Ok(JobDescription {
        title: first_heading(&raw),
        skills: crate::core::skills::extract_local_skills(&raw)
            .into_iter()
            .collect(),
        raw_text: raw,
    })
}

fn first_heading(markdown: &str) -> Option<String> {
    let mut in_heading = false;
    let mut text = String::new();

    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Heading { .. }) => in_heading = true,
            Event::End(_) if in_heading => return Some(text.trim().to_owned()),
            Event::Text(value) if in_heading => text.push_str(&value),
            _ => {}
        }
    }
    None
}

fn parse_skills_from_markdown(markdown: &str) -> Vec<String> {
    let mut capture = false;
    let mut values = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            capture = trimmed.to_lowercase().contains("skill");
            continue;
        }
        if capture && (trimmed.starts_with("- ") || trimmed.starts_with("* ")) {
            values.push(trimmed[2..].trim().to_owned());
        }
    }

    values
}

fn parse_experience_from_markdown(markdown: &str) -> Vec<Experience> {
    let mut experience = Vec::new();
    let mut current: Option<Experience> = None;
    let mut in_experience = false;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if in_experience && (trimmed.starts_with("## ") || trimmed.starts_with("### ")) {
            if let Some(item) = current.take() {
                experience.push(item);
            }
            let id = format!("exp-{}", experience.len() + 1);
            current = Some(Experience {
                id,
                company: Some(trimmed.trim_start_matches('#').trim().to_owned()),
                role: None,
                start: None,
                end: None,
                bullets: Vec::new(),
            });
        } else if trimmed.starts_with('#') {
            in_experience = trimmed.to_lowercase().contains("experience");
            continue;
        } else if !in_experience {
            continue;
        } else if let Some(item) = current.as_mut() {
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                item.bullets.push(trimmed[2..].trim().to_owned());
            }
        }
    }
    if let Some(item) = current {
        experience.push(item);
    }
    experience
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_yaml_frontmatter_cv() {
        let raw = r#"---
name: Alex Rivera
headline: Senior Rust Engineer
summary: Builds reliable systems.
skills:
  - Rust
  - Tokio
experience:
  - id: exp-1
    company: Northstar
    role: Engineer
    bullets:
      - Built Rust services with Tokio.
education:
  - institution: State University
    degree: B.S. Computer Science
    year: "2017"
---

Body
"#;

        let cv = parse_cv_markdown(raw).expect("CV should parse");

        assert_eq!(cv.name.as_deref(), Some("Alex Rivera"));
        assert_eq!(cv.skills, vec!["Rust", "Tokio"]);
        assert_eq!(cv.experience[0].id, "exp-1");
        assert_eq!(
            cv.education[0].degree.as_deref(),
            Some("B.S. Computer Science")
        );
    }

    #[test]
    fn falls_back_to_markdown_sections_when_frontmatter_is_absent() {
        let raw = r#"# Alex

## Skills

- Rust
- PostgreSQL

## Experience

### Northstar

- Built backend services.
"#;

        let cv = parse_cv_markdown(raw).expect("CV should parse");

        assert_eq!(cv.skills, vec!["Rust", "PostgreSQL"]);
        assert_eq!(cv.experience.len(), 1);
        assert_eq!(cv.experience[0].company.as_deref(), Some("Northstar"));
        assert_eq!(cv.experience[0].bullets, vec!["Built backend services."]);
    }
}
