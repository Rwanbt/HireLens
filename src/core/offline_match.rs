//! Offline matching engine — selects and ranks existing CV material against a
//! job, with **zero generation** (RFC §5.6/§8). It lives in `core` and never
//! depends on `llm`: the pipeline maps its result onto the LLM-shaped DTOs.
//!
//! Anti-hallucination is structural here: bullets are addressed by reference and
//! the original `String` is copied verbatim only at the output boundary, and
//! `prioritized_skills` is only ever a re-ordering of the caller's allowed set.

use std::collections::HashMap;

use crate::core::matching::weighted_requirements;
use crate::core::similarity::lexical_similarity;
use crate::core::skills::{extract_local_skills, normalize_skill, skill_category};
use crate::core::text::tokenize;
use crate::core::{Cv, Experience, JobDescription};

/// Weight of the skill signal in a bullet's relevance.
const ALPHA_SKILL: f32 = 1.0;
/// Weight of the lexical signal in a bullet's relevance.
const BETA_LEXICAL: f32 = 0.5;
/// Length-normalisation constant: a bullet with this many significant words
/// scores half of its long-form relevance, so a one-word bullet is penalised.
const LENGTH_NORM_K: f32 = 4.0;
/// Shortest token counted toward a bullet's length.
const MIN_BULLET_TOKEN_LEN: usize = 3;
/// Default bullets kept per experience for the optimised CV (RFC §12.7).
/// Demoted bullets are never deleted — the raw text / diff view keeps them.
const TOP_K_PER_EXPERIENCE: usize = 5;

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
/// Bullets are re-ordered by relevance to the job and the top-K per experience
/// are kept for the optimised CV (RFC §5.6/§12.7). Selection is by index into
/// the original `bullets`, so every emitted string is a verbatim copy.
pub fn run(cv: &Cv, job: &JobDescription, allowed_skills: &[String]) -> OfflineMatchResult {
    let requirement_weights = requirement_weight_map(job);
    OfflineMatchResult {
        prioritized_skills: prioritize_skills(job, allowed_skills),
        selected_bullets: select_bullets(cv, job, &requirement_weights),
    }
}

fn requirement_weight_map(job: &JobDescription) -> HashMap<String, f32> {
    weighted_requirements(job)
        .into_iter()
        .map(|requirement| (requirement.skill, requirement.weight))
        .collect()
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

/// For each experience, rank its bullets by relevance and keep the top-K. The
/// emitted bullet is `experience.bullets[index].clone()` — a verbatim copy,
/// never a reconstruction (anti-hal §8).
fn select_bullets(
    cv: &Cv,
    job: &JobDescription,
    weights: &HashMap<String, f32>,
) -> Vec<OfflineBullet> {
    let mut selected = Vec::new();
    for experience in &cv.experience {
        for index in top_bullet_indices(experience, job, weights) {
            selected.push(OfflineBullet {
                experience_id: experience.id.clone(),
                bullet: experience.bullets[index].clone(),
            });
        }
    }
    selected
}

/// Indices of an experience's bullets, ordered by relevance (desc), truncated to
/// the top-K. Ties fall back to the original order for determinism.
fn top_bullet_indices(
    experience: &Experience,
    job: &JobDescription,
    weights: &HashMap<String, f32>,
) -> Vec<usize> {
    let mut ranked: Vec<(usize, f32)> = experience
        .bullets
        .iter()
        .enumerate()
        .map(|(index, bullet)| (index, relevance(bullet, job, weights)))
        .collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    ranked
        .into_iter()
        .take(TOP_K_PER_EXPERIENCE)
        .map(|(index, _)| index)
        .collect()
}

/// `(alpha · job-skill-weight-in-bullet + beta · lexical_sim) × length_norm`
/// (RFC §5.6). Short and date-only bullets are penalised by `length_norm`.
fn relevance(bullet: &str, job: &JobDescription, weights: &HashMap<String, f32>) -> f32 {
    let skill_score: f32 = extract_local_skills(bullet)
        .iter()
        .filter_map(|skill| weights.get(skill))
        .sum();
    let lexical = lexical_similarity(bullet, &job.raw_text);
    (ALPHA_SKILL * skill_score + BETA_LEXICAL * lexical) * length_norm(bullet)
}

/// `n / (n + K)` over the count of significant, non-numeric words: 0 for an
/// empty or date-only bullet, rising toward 1 for substantive bullets. Pure
/// numbers (years, metrics standing alone) do not inflate length.
fn length_norm(bullet: &str) -> f32 {
    let words = tokenize(bullet)
        .iter()
        .filter(|token| {
            token.chars().count() >= MIN_BULLET_TOKEN_LEN && !token.chars().all(|c| c.is_numeric())
        })
        .count() as f32;
    words / (words + LENGTH_NORM_K)
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
    fn no_signal_preserves_order_and_keeps_bullets_verbatim() {
        // With no job signal every relevance ties at 0 → original order; the
        // canary token must survive verbatim (anti-hal §8).
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
    }

    #[test]
    fn relevant_bullet_ranks_above_irrelevant_one() {
        let cv = Cv {
            experience: vec![experience(
                "exp-1",
                &[
                    "Organised the annual office party.",
                    "Shipped Kubernetes operators in Rust for the platform team.",
                ],
            )],
            ..Cv::default()
        };
        let job = make_job(
            "We need Rust and Kubernetes experience.",
            &["rust", "kubernetes"],
        );

        let result = run(&cv, &job, &[]);
        // the job-relevant bullet must lead despite coming second in the CV
        assert!(result.selected_bullets[0]
            .bullet
            .contains("Kubernetes operators"));
    }

    #[test]
    fn short_bullet_is_penalised_against_a_rich_one() {
        let cv = Cv {
            experience: vec![experience(
                "exp-1",
                &[
                    "Rust.",
                    "Built fault-tolerant Rust services handling millions of requests.",
                ],
            )],
            ..Cv::default()
        };
        let job = make_job("Rust engineer wanted.", &["rust"]);

        let result = run(&cv, &job, &[]);
        // both mention Rust, but length_norm favours the substantive bullet
        assert_eq!(
            result.selected_bullets[0].bullet,
            "Built fault-tolerant Rust services handling millions of requests."
        );
    }

    #[test]
    fn top_k_limits_bullets_per_experience() {
        let many: Vec<String> = (0..7)
            .map(|i| format!("Delivered project number {i}."))
            .collect();
        let cv = Cv {
            experience: vec![Experience {
                id: "exp-1".into(),
                company: None,
                role: None,
                start: None,
                end: None,
                bullets: many,
            }],
            ..Cv::default()
        };
        let job = make_job("Delivered projects.", &[]);

        let result = run(&cv, &job, &[]);
        assert_eq!(result.selected_bullets.len(), TOP_K_PER_EXPERIENCE);
    }

    #[test]
    fn selected_bullets_are_a_verbatim_subset_of_the_cv() {
        let cv = Cv {
            experience: vec![
                experience("exp-1", &["Led Rust migration.", "Mentored juniors."]),
                experience("exp-2", &["Cut latency by 40% with caching."]),
            ],
            ..Cv::default()
        };
        let job = make_job("Rust and performance.", &["rust"]);

        let result = run(&cv, &job, &[]);
        // anti-hal property: strict equality against an original bullet, no edits
        for selected in &result.selected_bullets {
            let exists = cv
                .experience
                .iter()
                .filter(|e| e.id == selected.experience_id)
                .flat_map(|e| &e.bullets)
                .any(|original| *original == selected.bullet);
            assert!(
                exists,
                "selected bullet not verbatim in CV: {}",
                selected.bullet
            );
        }
    }

    fn make_job(raw_text: &str, skills: &[&str]) -> JobDescription {
        JobDescription {
            title: None,
            raw_text: raw_text.into(),
            skills: skills.iter().map(|s| s.to_string()).collect(),
        }
    }
}
