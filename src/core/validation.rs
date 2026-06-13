use anyhow::{bail, Result};
use hashbrown::HashSet;
use similar::{ChangeTag, TextDiff};

use crate::core::skills::{normalize_skill, skill_set};
use crate::core::Cv;
use crate::llm::AdaptationResponse;

pub fn validate_adaptation(cv: &Cv, adaptation: &AdaptationResponse) -> Result<()> {
    let allowed = skill_set(&cv.skills);
    reject_unknown_skills(&allowed, &adaptation.prioritized_skills)?;

    for adapted in &adaptation.selected_bullets {
        let original_exists = cv.experience.iter().any(|experience| {
            experience.id == adapted.experience_id
                && experience
                    .bullets
                    .iter()
                    .any(|bullet| bullet == &adapted.bullet)
        });
        if !original_exists {
            bail!(
                "adaptation referenced a bullet not present in the original CV: {}",
                adapted.bullet
            );
        }
    }

    Ok(())
}

fn reject_unknown_skills(allowed: &HashSet<String>, skills: &[String]) -> Result<()> {
    for skill in skills {
        let normalized = normalize_skill(skill);
        // C2 — reject empty / whitespace-only skills outright. Previously these
        // were skipped, letting a blank skill slip past the whitelist check.
        if normalized.is_empty() {
            bail!("LLM returned an empty or whitespace-only skill");
        }
        if !allowed.contains(&normalized) {
            bail!("LLM attempted to introduce unsupported skill: {}", skill);
        }
    }
    Ok(())
}

pub fn diff_markdown(original: &str, rendered: &str) -> String {
    let diff = TextDiff::from_lines(original, rendered);
    let mut output = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        output.push_str(sign);
        output.push_str(change.value());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Cv, Experience};
    use crate::llm::{AdaptationResponse, SelectedBullet};

    fn sample_cv() -> Cv {
        Cv {
            skills: vec!["Rust".into(), "Tokio".into()],
            experience: vec![Experience {
                id: "exp-1".into(),
                company: Some("Northstar".into()),
                role: Some("Engineer".into()),
                start: None,
                end: None,
                bullets: vec!["Built Rust services with Tokio.".into()],
            }],
            raw_markdown: "- Built Rust services with Tokio.\n".into(),
            ..Cv::default()
        }
    }

    #[test]
    fn rejects_skill_not_present_in_original_cv() {
        let adaptation = AdaptationResponse {
            prioritized_skills: vec!["Rust".into(), "Kubernetes".into()],
            selected_bullets: Vec::new(),
        };

        let error = validate_adaptation(&sample_cv(), &adaptation)
            .expect_err("unknown skill should be rejected");

        assert!(error.to_string().contains("unsupported skill: Kubernetes"));
    }

    #[test]
    fn rejects_bullet_not_present_in_original_cv() {
        let adaptation = AdaptationResponse {
            prioritized_skills: vec!["Rust".into()],
            selected_bullets: vec![SelectedBullet {
                experience_id: "exp-1".into(),
                bullet: "Invented a Kubernetes platform.".into(),
            }],
        };

        let error = validate_adaptation(&sample_cv(), &adaptation)
            .expect_err("invented bullet should be rejected");

        assert!(error
            .to_string()
            .contains("bullet not present in the original CV"));
    }

    #[test]
    fn rejects_empty_or_whitespace_skill() {
        let adaptation = AdaptationResponse {
            prioritized_skills: vec!["Rust".into(), "   ".into()],
            selected_bullets: Vec::new(),
        };

        let error = validate_adaptation(&sample_cv(), &adaptation)
            .expect_err("empty skill should be rejected");

        assert!(error.to_string().contains("empty or whitespace-only"));
    }

    #[test]
    fn rejects_bullet_with_unknown_experience_id() {
        let adaptation = AdaptationResponse {
            prioritized_skills: vec!["Rust".into()],
            selected_bullets: vec![SelectedBullet {
                // bullet text is verbatim, but no experience carries this id
                experience_id: "exp-999".into(),
                bullet: "Built Rust services with Tokio.".into(),
            }],
        };

        let error = validate_adaptation(&sample_cv(), &adaptation)
            .expect_err("bullet under unknown experience id should be rejected");

        assert!(error
            .to_string()
            .contains("bullet not present in the original CV"));
    }

    #[test]
    fn rejects_paraphrased_bullet() {
        let adaptation = AdaptationResponse {
            prioritized_skills: vec!["Rust".into()],
            selected_bullets: vec![SelectedBullet {
                experience_id: "exp-1".into(),
                // close paraphrase of the original — must still be rejected
                bullet: "Built Rust microservices using Tokio.".into(),
            }],
        };

        let error = validate_adaptation(&sample_cv(), &adaptation)
            .expect_err("paraphrased bullet should be rejected");

        assert!(error
            .to_string()
            .contains("bullet not present in the original CV"));
    }

    #[test]
    fn accepts_only_existing_skills_and_exact_bullets() {
        let adaptation = AdaptationResponse {
            prioritized_skills: vec!["Rust".into(), "Tokio".into()],
            selected_bullets: vec![SelectedBullet {
                experience_id: "exp-1".into(),
                bullet: "Built Rust services with Tokio.".into(),
            }],
        };

        validate_adaptation(&sample_cv(), &adaptation).expect("adaptation should be valid");
    }
}
