//! Offline matching engine — selects and ranks existing CV material against a
//! job, with **zero generation** (RFC §5.6/§8). It lives in `core` and never
//! depends on `llm`: the pipeline maps its result onto the LLM-shaped DTOs.
//!
//! Anti-hallucination is structural here: bullets are addressed by reference and
//! the original `String` is copied verbatim only at the output boundary, and
//! `prioritized_skills` is only ever a re-ordering of the caller's allowed set.

use crate::core::matching::weighted_requirements;
use crate::core::skills::{normalize_skill, skill_category};
use crate::core::{Cv, JobDescription};

/// Outcome of an offline match. A `core`-owned struct, deliberately NOT
/// `AdaptationResponse` (an LLM DTO) — `pipeline` maps one to the other.
#[derive(Debug, Clone)]
pub struct OfflineMatchResult {
    /// The caller's allowed skills, re-ordered by how strongly the job needs
    /// them. Always a permutation of the input — never an invented skill.
    pub prioritized_skills: Vec<String>,
    /// Bullets selected from the CV, copied verbatim.
    pub selected_bullets: Vec<OfflineBullet>,
}

/// A verbatim bullet tied to the experience it came from.
#[derive(Debug, Clone)]
pub struct OfflineBullet {
    pub experience_id: String,
    pub bullet: String,
}

/// Run the offline matcher.
///
/// `allowed_skills` is the anti-hallucination whitelist (the original CV
/// skills). `prioritized_skills` is only ever a re-ordering of it.
///
/// P1 scope: bullet *selection* is a verbatim passthrough preserving CV order —
/// real relevance ranking and top-K selection arrive in P3. The skill ordering,
/// however, already uses the job's weighted requirements.
pub fn run(cv: &Cv, job: &JobDescription, allowed_skills: &[String]) -> OfflineMatchResult {
    OfflineMatchResult {
        prioritized_skills: prioritize_skills(job, allowed_skills),
        selected_bullets: passthrough_bullets(cv),
    }
}

/// Order the allowed skills by job requirement weight (desc), breaking ties by
/// category priority then name, so the output is deterministic.
fn prioritize_skills(job: &JobDescription, allowed_skills: &[String]) -> Vec<String> {
    let weights = weighted_requirements(job);
    let weight_of = |skill: &str| -> f32 {
        let normalized = normalize_skill(skill);
        weights
            .iter()
            .find(|requirement| requirement.skill == normalized)
            .map(|requirement| requirement.weight)
            .unwrap_or(0.0)
    };

    let mut prioritized = allowed_skills.to_vec();
    prioritized.sort_by(|a, b| {
        weight_of(b)
            .partial_cmp(&weight_of(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| skill_category(a).cmp(&skill_category(b)))
            .then_with(|| a.cmp(b))
    });
    prioritized
}

/// Every bullet of the CV, in order, copied verbatim (anti-hal §8).
fn passthrough_bullets(cv: &Cv) -> Vec<OfflineBullet> {
    cv.experience
        .iter()
        .flat_map(|experience| {
            experience.bullets.iter().map(move |bullet| OfflineBullet {
                experience_id: experience.id.clone(),
                bullet: bullet.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Experience;

    fn experience(id: &str, bullets: &[&str]) -> Experience {
        Experience {
            id: id.into(),
            company: None,
            role: None,
            start: None,
            end: None,
            bullets: bullets.iter().map(|b| b.to_string()).collect(),
        }
    }

    #[test]
    fn prioritized_skills_is_a_permutation_of_allowed() {
        let cv = Cv {
            skills: vec!["Rust".into(), "Docker".into(), "Figma".into()],
            ..Cv::default()
        };
        let job = make_job("rust rust docker", &["rust", "docker"]);
        let allowed = cv.skills.clone();

        let result = run(&cv, &job, &allowed);

        // same multiset, no skill invented or dropped
        let mut got = result.prioritized_skills.clone();
        let mut expected = allowed.clone();
        got.sort();
        expected.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn prioritized_skills_lead_with_the_most_required() {
        let cv = Cv {
            skills: vec!["Figma".into(), "Rust".into(), "Docker".into()],
            ..Cv::default()
        };
        // rust is required most often; figma not mentioned at all.
        let job = make_job("rust rust rust docker", &["rust", "docker"]);

        let result = run(&cv, &job, &cv.skills);

        assert_eq!(
            result.prioritized_skills.first().map(String::as_str),
            Some("Rust")
        );
        assert_eq!(
            result.prioritized_skills.last().map(String::as_str),
            Some("Figma")
        );
    }

    #[test]
    fn bullets_are_passed_through_verbatim_and_in_order() {
        // canary token must survive verbatim (anti-hal §8)
        let cv = Cv {
            experience: vec![
                experience(
                    "exp-1",
                    &["Built ZZQX-canary pipelines.", "Led a team of five."],
                ),
                experience("exp-2", &["Shipped a billing platform."]),
            ],
            ..Cv::default()
        };
        let job = make_job("anything", &[]);

        let result = run(&cv, &job, &[]);

        let bullets: Vec<&str> = result
            .selected_bullets
            .iter()
            .map(|b| b.bullet.as_str())
            .collect();
        assert_eq!(
            bullets,
            vec![
                "Built ZZQX-canary pipelines.",
                "Led a team of five.",
                "Shipped a billing platform.",
            ]
        );
        assert_eq!(result.selected_bullets[0].experience_id, "exp-1");
        assert_eq!(result.selected_bullets[2].experience_id, "exp-2");
    }

    fn make_job(raw_text: &str, skills: &[&str]) -> JobDescription {
        JobDescription {
            title: None,
            raw_text: raw_text.into(),
            skills: skills.iter().map(|s| s.to_string()).collect(),
        }
    }
}
