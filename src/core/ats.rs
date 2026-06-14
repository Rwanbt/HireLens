use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

use crate::core::matching::{count_skill_occurrences, keyword_coverage, ScoreReason, SkillStatus};
use crate::core::similarity::lexical_similarity;
use crate::core::skills::{normalize_skill, skill_set};
use crate::core::{Cv, JobDescription};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub score: AtsScore,
    pub cv_skills: Vec<String>,
    pub job_skills: Vec<String>,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub explanations: Vec<ScoreReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtsScore {
    pub skill_match_ratio: f32,
    pub score: u8,
    /// Share of job keywords literally present in the CV text (0..=1).
    #[serde(default)]
    pub keyword_score: f32,
    /// Share of core ATS sections (skills, experience, education) present (0..=1).
    #[serde(default)]
    pub structure_score: f32,
    /// Lexical similarity between the job and the CV text (0..=1).
    #[serde(default)]
    pub lexical_score: f32,
}

pub fn compute_audit(cv: &Cv, job: &JobDescription) -> AuditReport {
    let cv_skills = skill_set(&cv.skills);
    let job_skills = skill_set(&job.skills);
    let matched = intersection(&cv_skills, &job_skills);
    let missing = difference(&job_skills, &cv_skills);
    let ratio = if job_skills.is_empty() {
        1.0
    } else {
        matched.len() as f32 / job_skills.len() as f32
    };

    let signals = count_skill_occurrences(job);
    let explanations = signals
        .into_iter()
        .map(|sig| {
            let status = if cv_skills.contains(&sig.skill) {
                SkillStatus::Present
            } else if sig.occurrences >= 2 {
                SkillStatus::Missing
            } else {
                SkillStatus::Weak
            };
            ScoreReason {
                skill: sig.skill,
                status,
                occurrences: sig.occurrences,
            }
        })
        .collect();

    let keyword_score = keyword_coverage(&job.raw_text, &cv.raw_markdown);
    let structure_score = structure_completeness(cv);
    let lexical_score = lexical_similarity(&job.raw_text, &cv.raw_markdown);

    AuditReport {
        score: AtsScore {
            skill_match_ratio: ratio,
            score: (ratio * 100.0).round().clamp(0.0, 100.0) as u8,
            keyword_score,
            structure_score,
            lexical_score,
        },
        cv_skills: sorted(cv_skills),
        job_skills: sorted(job_skills),
        matched_skills: matched,
        missing_skills: missing,
        explanations,
    }
}

pub fn merge_skills(primary: &[String], secondary: &[String]) -> Vec<String> {
    let mut merged = skill_set(primary);
    merged.extend(secondary.iter().map(|skill| normalize_skill(skill)));
    sorted(merged)
}

fn intersection(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
    let values = left.intersection(right).cloned().collect();
    sorted(values)
}

fn difference(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
    let values = left.difference(right).cloned().collect();
    sorted(values)
}

fn sorted(values: HashSet<String>) -> Vec<String> {
    let mut values: Vec<_> = values
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect();
    values.sort();
    values
}

/// Fraction of the three core ATS sections (skills, experience, education)
/// present in the parsed CV.
fn structure_completeness(cv: &Cv) -> f32 {
    const CORE_SECTIONS: f32 = 3.0;
    let mut present = 0.0;
    if !cv.skills.is_empty() {
        present += 1.0;
    }
    if !cv.experience.is_empty() {
        present += 1.0;
    }
    if !cv.education.is_empty() {
        present += 1.0;
    }
    present / CORE_SECTIONS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Cv, Experience};

    #[test]
    fn computes_skill_match_score_and_missing_skills() {
        let cv = Cv {
            skills: vec!["Rust".into(), "Tokio".into(), "Docker".into()],
            ..Cv::default()
        };
        let job = JobDescription {
            title: Some("Platform Engineer".into()),
            raw_text: String::new(),
            skills: vec!["rust".into(), "kubernetes".into(), "docker".into()],
        };

        let report = compute_audit(&cv, &job);

        assert_eq!(report.score.score, 67);
        assert_eq!(report.matched_skills, vec!["docker", "rust"]);
        assert_eq!(report.missing_skills, vec!["kubernetes"]);
    }

    #[test]
    fn explanations_classify_present_missing_weak() {
        let cv = Cv {
            skills: vec!["Rust".into()],
            ..Cv::default()
        };
        let job = JobDescription {
            title: None,
            // docker appears twice (Missing), kubernetes once (Weak)
            raw_text: "rust docker docker kubernetes".into(),
            skills: vec!["rust".into(), "docker".into(), "kubernetes".into()],
        };

        let report = compute_audit(&cv, &job);
        let by_skill = |name: &str| {
            report
                .explanations
                .iter()
                .find(|reason| reason.skill == name)
                .unwrap_or_else(|| panic!("expected explanation for {name}"))
                .clone()
        };

        // in the CV → Present, regardless of occurrence count in the job
        assert_eq!(by_skill("rust").status, SkillStatus::Present);

        let docker = by_skill("docker");
        assert_eq!(docker.status, SkillStatus::Missing);
        assert_eq!(docker.occurrences, 2);

        let kubernetes = by_skill("kubernetes");
        assert_eq!(kubernetes.status, SkillStatus::Weak);
        assert_eq!(kubernetes.occurrences, 1);
    }

    #[test]
    fn computes_keyword_and_structure_scores() {
        let cv = Cv {
            skills: vec!["Rust".into()],
            experience: vec![Experience {
                id: "exp-1".into(),
                company: None,
                role: None,
                start: None,
                end: None,
                bullets: vec![],
            }],
            raw_markdown: "Rust developer on a payments platform".into(),
            ..Cv::default()
        };
        let job = JobDescription {
            title: None,
            // non-skill keywords: payments, platform, billing, dashboards (4).
            // CV covers payments + platform → 0.5.
            raw_text: "Payments platform billing dashboards.".into(),
            skills: vec!["rust".into(), "docker".into()],
        };

        let report = compute_audit(&cv, &job);

        // 2 of 4 job keywords present in the CV markdown → 0.5
        assert!((report.score.keyword_score - 0.5).abs() < 1e-6);
        // skills + experience present, education missing → 2/3
        assert!((report.score.structure_score - 2.0 / 3.0).abs() < 1e-6);
    }
}
