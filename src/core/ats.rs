use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

use crate::core::matching::{count_skill_occurrences, ScoreReason, SkillStatus};
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

    AuditReport {
        score: AtsScore {
            skill_match_ratio: ratio,
            score: (ratio * 100.0).round().clamp(0.0, 100.0) as u8,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Cv;

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
}
