pub mod typst_render;

use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::core::Cv;
use crate::llm::AdaptationResponse;

pub struct MarkdownExporter;

impl MarkdownExporter {
    pub fn write(markdown: &str, path: &Path) -> Result<()> {
        std::fs::write(path, markdown)
            .with_context(|| format!("failed to write {}", path.display()))
    }
}

pub struct PdfExporter;

impl PdfExporter {
    pub async fn write_with_pandoc(markdown_path: &Path, pdf_path: &Path) -> Result<()> {
        let status = Command::new("pandoc")
            .arg(markdown_path)
            .arg("-o")
            .arg(pdf_path)
            .status()
            .await
            .context("failed to execute pandoc; install pandoc or omit --pdf")?;

        if !status.success() {
            anyhow::bail!("pandoc failed with status {}", status);
        }
        Ok(())
    }
}

pub fn render_cv(cv: &Cv, adaptation: Option<&AdaptationResponse>) -> String {
    let mut out = String::new();
    if let Some(name) = &cv.name {
        out.push_str("# ");
        out.push_str(name);
        out.push_str("\n\n");
    }
    if let Some(headline) = cv.headline.as_ref() {
        out.push_str(headline);
        out.push_str("\n\n");
    }
    if let Some(summary) = cv.summary.as_ref() {
        out.push_str("## Summary\n\n");
        out.push_str(summary);
        out.push_str("\n\n");
    }

    let skills = adaptation
        .map(|a| a.prioritized_skills.as_slice())
        .unwrap_or(cv.skills.as_slice());
    if !skills.is_empty() {
        out.push_str("## Skills\n\n");
        for skill in skills {
            out.push_str("- ");
            out.push_str(skill);
            out.push('\n');
        }
        out.push('\n');
    }

    if !cv.experience.is_empty() {
        out.push_str("## Experience\n\n");
        for experience in &cv.experience {
            out.push_str("### ");
            out.push_str(
                experience
                    .role
                    .as_deref()
                    .or(experience.company.as_deref())
                    .unwrap_or("Experience"),
            );
            out.push_str("\n\n");
            if let Some(company) = &experience.company {
                out.push_str(company);
                out.push_str("\n\n");
            }
            for bullet in &experience.bullets {
                let selected = adaptation
                    .map(|a| {
                        a.selected_bullets.iter().any(|adapted| {
                            adapted.experience_id == experience.id && adapted.bullet == *bullet
                        })
                    })
                    .unwrap_or(true);
                if !selected {
                    continue;
                }
                out.push_str("- ");
                out.push_str(bullet);
                out.push('\n');
            }
            out.push('\n');
        }
    }

    if !cv.education.is_empty() {
        out.push_str("## Education\n\n");
        for education in &cv.education {
            out.push_str("- ");
            out.push_str(
                education
                    .degree
                    .as_deref()
                    .or(education.institution.as_deref())
                    .unwrap_or("Education"),
            );
            if let Some(institution) = &education.institution {
                out.push_str(", ");
                out.push_str(institution);
            }
            if let Some(year) = &education.year {
                out.push_str(" (");
                out.push_str(year);
                out.push(')');
            }
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Cv, Experience};
    use crate::llm::{AdaptationResponse, SelectedBullet};

    #[test]
    fn renders_only_selected_original_bullets_for_adapted_cv() {
        let cv = Cv {
            name: Some("Alex Rivera".into()),
            headline: Some("Senior Rust Engineer".into()),
            summary: Some("Builds reliable systems.".into()),
            skills: vec!["Rust".into(), "Tokio".into(), "Docker".into()],
            experience: vec![Experience {
                id: "exp-1".into(),
                company: Some("Northstar".into()),
                role: Some("Engineer".into()),
                start: None,
                end: None,
                bullets: vec![
                    "Built Rust services with Tokio.".into(),
                    "Maintained Docker deployments.".into(),
                ],
            }],
            ..Cv::default()
        };
        let adaptation = AdaptationResponse {
            prioritized_skills: vec!["Rust".into(), "Tokio".into()],
            selected_bullets: vec![SelectedBullet {
                experience_id: "exp-1".into(),
                bullet: "Built Rust services with Tokio.".into(),
            }],
        };

        let rendered = render_cv(&cv, Some(&adaptation));

        assert!(rendered.contains("- Rust"));
        assert!(rendered.contains("- Built Rust services with Tokio."));
        assert!(!rendered.contains("Maintained Docker deployments."));
    }
}
