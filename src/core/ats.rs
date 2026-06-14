use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

use crate::core::matching::{
    count_skill_occurrences, keyword_coverage, weighted_requirements, ScoreReason, SkillStatus,
};
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
    let sections = present_sections(cv);
    let structure_score = sections as f32 / CORE_SECTIONS;
    let lexical_score = lexical_similarity(&job.raw_text, &cv.raw_markdown);

    // skill_cov is weighted by the job's requirement weights (RFC §5.5), not a
    // raw count ratio. No dictionary skill in the job → non-tech blend.
    let requirements = weighted_requirements(job);
    let total_weight: f32 = requirements.iter().map(|req| req.weight).sum();
    let matched_weight: f32 = requirements
        .iter()
        .filter(|req| cv_skills.contains(&req.skill))
        .map(|req| req.weight)
        .sum();
    let non_tech = total_weight == 0.0;
    let skill_cov = if non_tech {
        0.0
    } else {
        matched_weight / total_weight
    };

    let score = if is_empty_input(cv, job) {
        0
    } else {
        blend(&MatchSignals {
            skill_cov,
            keyword_cov: keyword_score,
            lexical_sim: lexical_score,
            structure_factor: structure_factor(sections),
            non_tech,
        })
    };

    AuditReport {
        score: AtsScore {
            skill_match_ratio: ratio,
            score,
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

/// Total core ATS sections scored (skills, experience, education).
const CORE_SECTIONS: f32 = 3.0;

/// Blend weights for a tech job (Σ = 1). Kept as centralised constants on
/// purpose — runtime config of these is deferred until an annotated eval set
/// exists to tune against (see ADR-0008, "Alternatives rejetées").
const W_SKILL: f32 = 0.45;
const W_KEYWORD: f32 = 0.15;
const W_LEXICAL: f32 = 0.40;
/// Non-tech fallback: the skill dimension is removed and redistributed onto the
/// lexical + keyword signals (RFC §5.5).
const W_NONTECH_LEXICAL: f32 = 0.80;
const W_NONTECH_KEYWORD: f32 = 0.20;
/// In the non-tech fallback, a base below this collapses to 0 — guards against
/// false positives when there is too little signal (RFC §0.2, Mistral).
const NONTECH_FLOOR: f32 = 0.20;

/// The four orthogonal match signals (RFC §5.5). Each ∈ [0,1] except
/// `structure_factor`, a multiplicative gate that can only *reduce* the score.
struct MatchSignals {
    skill_cov: f32,
    keyword_cov: f32,
    lexical_sim: f32,
    structure_factor: f32,
    /// The job declares no dictionary skill → use the non-tech blend.
    non_tech: bool,
}

/// Blend the signals into a 0..=100 score: a weighted sum gated multiplicatively
/// by structure (RFC §5.5). Structure can shave at most 25 %; it never lifts a
/// weak match.
fn blend(signals: &MatchSignals) -> u8 {
    debug_assert!((W_SKILL + W_KEYWORD + W_LEXICAL - 1.0).abs() < 1e-6);

    let base = if signals.non_tech {
        let base =
            W_NONTECH_LEXICAL * signals.lexical_sim + W_NONTECH_KEYWORD * signals.keyword_cov;
        if base < NONTECH_FLOOR {
            return 0;
        }
        base
    } else {
        W_SKILL * signals.skill_cov
            + W_KEYWORD * signals.keyword_cov
            + W_LEXICAL * signals.lexical_sim
    };

    (base * signals.structure_factor * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8
}

/// Number of core ATS sections present (skills, experience, education).
fn present_sections(cv: &Cv) -> u8 {
    u8::from(!cv.skills.is_empty())
        + u8::from(!cv.experience.is_empty())
        + u8::from(!cv.education.is_empty())
}

/// Structure gate ∈ {1.0, 0.9, 0.75} — tightened so structure cannot compensate
/// for a weak match (RFC §0.2, max −25 %).
fn structure_factor(sections: u8) -> f32 {
    match sections {
        3 => 1.0,
        2 => 0.9,
        _ => 0.75,
    }
}

/// Either side empty (no skills and no text) → no meaningful score.
fn is_empty_input(cv: &Cv, job: &JobDescription) -> bool {
    let job_empty = job.skills.is_empty() && job.raw_text.trim().is_empty();
    let cv_empty = cv.skills.is_empty() && cv.raw_markdown.trim().is_empty();
    job_empty || cv_empty
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

        // legacy raw ratio unchanged; the blended total is exercised separately.
        assert!((report.score.skill_match_ratio - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(report.matched_skills, vec!["docker", "rust"]);
        assert_eq!(report.missing_skills, vec!["kubernetes"]);
        assert!(report.score.score > 0 && report.score.score <= 100);
    }

    #[test]
    fn blend_applies_weights_and_structure_gate() {
        let full = MatchSignals {
            skill_cov: 1.0,
            keyword_cov: 1.0,
            lexical_sim: 1.0,
            structure_factor: 1.0,
            non_tech: false,
        };
        assert_eq!(blend(&full), 100);

        // structure gate shaves at most 25 %
        let gated = MatchSignals {
            structure_factor: 0.75,
            ..full
        };
        assert_eq!(blend(&gated), 75);

        // weights: skill-only match → 0.45
        let skill_only = MatchSignals {
            skill_cov: 1.0,
            keyword_cov: 0.0,
            lexical_sim: 0.0,
            structure_factor: 1.0,
            non_tech: false,
        };
        assert_eq!(blend(&skill_only), 45);
    }

    #[test]
    fn non_tech_blend_has_a_floor() {
        // too little signal → collapses to 0
        let weak = MatchSignals {
            skill_cov: 0.0,
            keyword_cov: 0.1,
            lexical_sim: 0.1,
            structure_factor: 1.0,
            non_tech: true,
        };
        assert_eq!(blend(&weak), 0);

        // above the floor → scored from lexical (0.8) + keyword (0.2)
        let ok = MatchSignals {
            skill_cov: 0.0,
            keyword_cov: 1.0,
            lexical_sim: 1.0,
            structure_factor: 1.0,
            non_tech: true,
        };
        assert_eq!(blend(&ok), 100);
    }

    #[test]
    fn golden_non_tech_french_pair() {
        let job = JobDescription {
            title: Some("Chargé de marketing".into()),
            raw_text: "Recherche un chargé de marketing digital pour piloter les campagnes \
                       publicitaires et la stratégie de contenu sur les réseaux sociaux."
                .into(),
            skills: vec![], // non-tech: no dictionary skill
        };
        let strong_cv = Cv {
            skills: vec!["communication".into()],
            experience: vec![Experience {
                id: "exp-1".into(),
                company: None,
                role: None,
                start: None,
                end: None,
                bullets: vec![],
            }],
            raw_markdown: "Chargé de marketing digital, j'ai piloté des campagnes publicitaires \
                           et défini la stratégie de contenu sur les réseaux sociaux."
                .into(),
            ..Cv::default()
        };
        let weak_cv = Cv {
            raw_markdown: "Plombier expérimenté en installation sanitaire et chauffage.".into(),
            ..Cv::default()
        };

        let strong = compute_audit(&strong_cv, &job).score.score;
        let weak = compute_audit(&weak_cv, &job).score.score;

        assert!(strong >= 25, "strong non-tech match should score: {strong}");
        assert_eq!(weak, 0, "unrelated CV should hit the non-tech floor");
        assert!(strong > weak);
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

    // ── P4: golden fixtures (expected bands) + property tests ──────────────

    fn cv_with(skills: &[&str], markdown: &str) -> Cv {
        Cv {
            skills: skills.iter().map(|s| s.to_string()).collect(),
            experience: vec![Experience {
                id: "exp-1".into(),
                company: None,
                role: None,
                start: None,
                end: None,
                bullets: vec!["Delivered production systems.".into()],
            }],
            education: vec![crate::core::Education {
                institution: Some("State University".into()),
                degree: Some("BSc".into()),
                year: Some("2016".into()),
            }],
            raw_markdown: markdown.into(),
            ..Cv::default()
        }
    }

    fn job_with(skills: &[&str], raw_text: &str) -> JobDescription {
        JobDescription {
            title: None,
            raw_text: raw_text.into(),
            skills: skills.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn golden_tech_strong_beats_weak() {
        let job = job_with(
            &["rust", "kubernetes", "postgresql"],
            "Backend engineer in Rust deploying Kubernetes services backed by PostgreSQL.",
        );
        let strong = cv_with(
            &["rust", "kubernetes", "postgresql"],
            "Senior Rust backend engineer running Kubernetes clusters with PostgreSQL.",
        );
        let weak = cv_with(
            &["php"],
            "WordPress site builder writing PHP themes for small businesses.",
        );

        let strong_score = compute_audit(&strong, &job).score.score;
        let weak_score = compute_audit(&weak, &job).score.score;

        assert!(strong_score >= 60, "strong tech match: {strong_score}");
        assert!(weak_score <= 25, "weak tech match: {weak_score}");
        assert!(strong_score > weak_score);
    }

    #[test]
    fn golden_tech_french_pair_scores() {
        let job = job_with(
            &["python", "docker", "aws"],
            "Ingénieur Python requis pour conteneuriser avec Docker et déployer sur AWS.",
        );
        let cv = cv_with(
            &["python", "docker", "aws"],
            "Développeur Python expérimenté en Docker et déploiement AWS.",
        );
        let score = compute_audit(&cv, &job).score.score;
        assert!(score >= 50, "FR tech match should score well: {score}");
    }

    #[test]
    fn scores_are_bounded_and_idempotent() {
        let cases = [
            (
                cv_with(&["rust"], "Rust everywhere"),
                job_with(&["rust"], "Rust role"),
            ),
            (
                cv_with(&[], "Marketing lead"),
                job_with(&[], "Marketing strategy and content"),
            ),
            (
                Cv::default(),
                JobDescription {
                    title: None,
                    raw_text: String::new(),
                    skills: vec![],
                },
            ),
        ];
        for (cv, job) in &cases {
            let a = compute_audit(cv, job);
            let b = compute_audit(cv, job);
            // bounds
            assert!(a.score.score <= 100);
            for sub in [
                a.score.keyword_score,
                a.score.structure_score,
                a.score.lexical_score,
                a.score.skill_match_ratio,
            ] {
                assert!((0.0..=1.0).contains(&sub), "sub-score out of range: {sub}");
            }
            // idempotence: same inputs → identical serialised report
            let ja = serde_json::to_string(&a).unwrap();
            let jb = serde_json::to_string(&b).unwrap();
            assert_eq!(ja, jb);
        }
    }
}
