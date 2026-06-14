use regex::Regex;

use crate::core::skills::normalize_skill;
use crate::core::JobDescription;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SkillStatus {
    Present,
    Missing,
    Weak,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScoreReason {
    pub skill: String,
    pub status: SkillStatus,
    pub occurrences: usize,
}

/// Combien de fois un skill apparaît dans le texte brut de l'offre.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillSignal {
    pub skill: String,
    pub occurrences: usize,
}

/// Pour chaque skill de l'offre, compte ses occurrences dans `job.raw_text`.
pub fn count_skill_occurrences(job: &JobDescription) -> Vec<SkillSignal> {
    let haystack = job.raw_text.to_lowercase();
    job.skills
        .iter()
        .map(|raw| {
            let skill = normalize_skill(raw);
            // Word-boundary regex prevents "rust" from matching inside "frustrated".
            // regex::escape handles skills with special chars like "C++" or "C#".
            let occurrences = if skill.is_empty() {
                0
            } else {
                let pattern = format!(r"\b{}\b", regex::escape(&skill));
                Regex::new(&pattern)
                    .map(|re| re.find_iter(&haystack).count())
                    .unwrap_or(0)
            };
            SkillSignal { skill, occurrences }
        })
        .collect()
}

/// Fraction of `skills` that appear (word-boundary, case-insensitive) in `text`.
/// Returns 1.0 for an empty skill list (nothing required → fully covered).
pub fn keyword_coverage(skills: &[String], text: &str) -> f32 {
    if skills.is_empty() {
        return 1.0;
    }
    let haystack = text.to_lowercase();
    let present = skills
        .iter()
        .filter(|raw| {
            let skill = normalize_skill(raw);
            if skill.is_empty() {
                return false;
            }
            let pattern = format!(r"\b{}\b", regex::escape(&skill));
            Regex::new(&pattern)
                .map(|re| re.is_match(&haystack))
                .unwrap_or(false)
        })
        .count();
    present as f32 / skills.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_job(raw_text: &str, skills: Vec<&str>) -> JobDescription {
        JobDescription {
            title: None,
            raw_text: raw_text.to_string(),
            skills: skills.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn count_skill_occurrences_respects_word_boundary() {
        // "rust" inside "frustrated" must NOT match
        let job = make_job(
            "I am frustrated with this job. No Rust mention.",
            vec!["rust"],
        );
        let signals = count_skill_occurrences(&job);
        assert_eq!(
            signals[0].occurrences, 1,
            "'rust' should match 'Rust' but not 'frust-rated'"
        );
    }

    #[test]
    fn count_skill_occurrences_counts_correctly() {
        let job = make_job(
            "We need rust developers. Rust experience is required.",
            vec!["Rust"],
        );
        let signals = count_skill_occurrences(&job);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].skill, "rust");
        assert_eq!(signals[0].occurrences, 2);
    }

    #[test]
    fn count_skill_occurrences_zero_for_absent_skill() {
        let job = make_job("We need Python and Django expertise.", vec!["rust"]);
        let signals = count_skill_occurrences(&job);
        assert_eq!(signals[0].occurrences, 0);
    }

    #[test]
    fn count_skill_occurrences_empty_skill_yields_zero() {
        let job = make_job("Some job text", vec!["  "]);
        let signals = count_skill_occurrences(&job);
        assert_eq!(signals[0].occurrences, 0);
    }

    #[test]
    fn count_skill_occurrences_multiple_skills() {
        let job = make_job(
            "docker docker docker kubernetes kubernetes",
            vec!["Docker", "Kubernetes", "Rust"],
        );
        let signals = count_skill_occurrences(&job);
        assert_eq!(signals[0].occurrences, 3);
        assert_eq!(signals[1].occurrences, 2);
        assert_eq!(signals[2].occurrences, 0);
    }

    #[test]
    fn keyword_coverage_is_ratio_of_present_skills() {
        let skills = vec!["Rust".into(), "Docker".into(), "Kubernetes".into()];
        let coverage = keyword_coverage(&skills, "Built Rust apps with Docker.");
        assert!((coverage - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn keyword_coverage_empty_skills_is_full() {
        assert_eq!(keyword_coverage(&[], "anything"), 1.0);
    }

    #[test]
    fn keyword_coverage_respects_word_boundary() {
        let skills = vec!["Rust".into()];
        assert_eq!(keyword_coverage(&skills, "I am frustrated."), 0.0);
    }
}
