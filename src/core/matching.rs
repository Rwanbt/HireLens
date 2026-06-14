use std::collections::HashMap;

use hashbrown::HashSet;
use regex::Regex;

use crate::core::skills::{normalize_skill, skill_words};
use crate::core::text::{is_stopword, tokenize_words};
use crate::core::JobDescription;

/// How many top keywords represent the job (RFC §0.2 — Top-10–15).
const MAX_KEYWORDS: usize = 15;
/// Shortest token kept as a keyword (drops `ci`, `qa`, noise).
const MIN_KEYWORD_LEN: usize = 3;
/// Per-token frequency cap so one repeated word cannot dominate the ranking.
const MAX_KEYWORD_OCCURRENCES: usize = 3;

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

/// Top job-specific keywords: the most frequent non-stopword, non-skill tokens
/// of the job text (RFC §0.2/§5.5). Pure function, orthogonal to the skill
/// dimension — it never re-counts the skill signal, so the two stay independent.
pub fn extract_keywords(job_text: &str) -> Vec<String> {
    let skill_words = skill_words();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for token in tokenize_words(job_text) {
        if token.len() < MIN_KEYWORD_LEN || is_stopword(&token) || skill_words.contains(&token) {
            continue;
        }
        let count = counts.entry(token).or_insert(0);
        if *count < MAX_KEYWORD_OCCURRENCES {
            *count += 1;
        }
    }
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    // Frequency desc, then alphabetical so ties are deterministic.
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(MAX_KEYWORDS)
        .map(|(token, _)| token)
        .collect()
}

/// Share of the job's top keywords that literally appear in the CV text (0..=1).
/// An empty keyword set (e.g. empty job text) yields 1.0 — nothing to cover.
pub fn keyword_coverage(job_text: &str, cv_text: &str) -> f32 {
    let keywords = extract_keywords(job_text);
    if keywords.is_empty() {
        return 1.0;
    }
    let cv_tokens: HashSet<String> = tokenize_words(cv_text).into_iter().collect();
    let present = keywords
        .iter()
        .filter(|keyword| cv_tokens.contains(*keyword))
        .count();
    present as f32 / keywords.len() as f32
}

/// Multiplier applied when a required skill also appears in the job title.
const TITLE_BOOST: f32 = 1.5;
/// Multiplier when a "required / must-have" cue sits near the skill mention.
const REQUIRED_BOOST: f32 = 1.4;
/// Multiplier when a "nice-to-have" cue sits near the skill mention.
const NICE_TO_HAVE_FACTOR: f32 = 0.6;
/// Frequency saturation constant for `1 - e^(-n/k)` (diminishing returns).
const FREQ_SATURATION_K: f32 = 2.0;
/// Occurrence cap before saturation, so spam cannot inflate a requirement.
const MAX_REQ_OCCURRENCES: usize = 5;

/// "Required / must-have" cues, FR+EN, accent-folded lowercase.
const REQUIRED_CUES: &[&str] = &[
    "required",
    "must",
    "mandatory",
    "requis",
    "obligatoire",
    "indispensable",
    "essentiel",
    "exige",
];
/// "Nice-to-have / optional" cues, FR+EN, accent-folded lowercase.
const NICE_TO_HAVE_CUES: &[&str] = &[
    "nice-to-have",
    "nice to have",
    "bonus",
    "optional",
    "optionnel",
    "souhaite",
    "apprecie",
    "preferred",
    "un plus",
];

/// A job requirement with its computed weight.
#[derive(Debug, Clone, PartialEq)]
pub struct RequirementWeight {
    pub skill: String,
    pub weight: f32,
}

/// Weight every job requirement: `saturate(frequency) × title_boost ×
/// section_boost` (RFC §5.4). Pure function. Returned sorted by weight
/// descending (alphabetical tie-break) for determinism.
pub fn weighted_requirements(job: &JobDescription) -> Vec<RequirementWeight> {
    let title = fold_lower(job.title.as_deref().unwrap_or_default());
    let haystack = fold_lower(&job.raw_text);

    let mut weights: Vec<RequirementWeight> = count_skill_occurrences(job)
        .into_iter()
        .filter(|signal| !signal.skill.is_empty())
        .map(|signal| {
            let frequency = saturate(signal.occurrences);
            let title_boost = if title.contains(&signal.skill) {
                TITLE_BOOST
            } else {
                1.0
            };
            let section_boost = section_boost(&haystack, &signal.skill);
            RequirementWeight {
                weight: frequency * title_boost * section_boost,
                skill: signal.skill,
            }
        })
        .collect();

    weights.sort_by(|a, b| {
        b.weight
            .partial_cmp(&a.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.skill.cmp(&b.skill))
    });
    weights
}

fn fold_lower(text: &str) -> String {
    crate::core::text::fold_accents(&text.to_lowercase())
}

/// `1 - e^(-n/k)`: 0 at n=0, rising with diminishing returns, capped at
/// `MAX_REQ_OCCURRENCES`. A listed requirement counts at least once.
fn saturate(occurrences: usize) -> f32 {
    let capped = occurrences.clamp(1, MAX_REQ_OCCURRENCES) as f32;
    1.0 - (-capped / FREQ_SATURATION_K).exp()
}

/// Inspect the clause holding the skill's first mention for a required /
/// optional cue. Scoping to the clause (not a fixed char window) stops a cue
/// from one sentence leaking into an adjacent skill's score. `haystack` and
/// `skill` are both accent-folded lowercase.
fn section_boost(haystack: &str, skill: &str) -> f32 {
    let clause = clause_containing(haystack, skill).unwrap_or(haystack);
    if REQUIRED_CUES.iter().any(|cue| clause.contains(cue)) {
        REQUIRED_BOOST
    } else if NICE_TO_HAVE_CUES.iter().any(|cue| clause.contains(cue)) {
        NICE_TO_HAVE_FACTOR
    } else {
        1.0
    }
}

/// The clause (between `.!?;` or newlines) containing the skill's first mention.
fn clause_containing<'a>(haystack: &'a str, skill: &str) -> Option<&'a str> {
    const BOUNDARIES: [char; 5] = ['.', '!', '?', ';', '\n'];
    let pos = haystack.find(skill)?;
    let start = haystack[..pos]
        .rfind(BOUNDARIES)
        .map(|index| index + 1)
        .unwrap_or(0);
    let after = pos + skill.len();
    let end = haystack[after..]
        .find(BOUNDARIES)
        .map(|index| after + index)
        .unwrap_or(haystack.len());
    Some(&haystack[start..end])
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
    fn extract_keywords_drops_stopwords_and_skills() {
        // "rust"/"docker" are skills, "the"/"for"/"a" are stopwords → excluded.
        let keywords = extract_keywords("The payments platform for a rust and docker team.");
        assert!(keywords.contains(&"payments".to_string()));
        assert!(keywords.contains(&"platform".to_string()));
        assert!(keywords.contains(&"team".to_string()));
        assert!(!keywords.contains(&"rust".to_string()));
        assert!(!keywords.contains(&"docker".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
    }

    #[test]
    fn keyword_coverage_is_ratio_present_in_cv() {
        // job keywords: payments, platform, billing, dashboards (4 — no stopword/skill)
        let job = "Payments platform billing dashboards.";
        // CV mentions payments + platform only → 2 of 4 present.
        let coverage = keyword_coverage(job, "Built a payments platform.");
        assert!((coverage - 0.5).abs() < 1e-6, "got {coverage}");
    }

    #[test]
    fn keyword_coverage_empty_job_is_full() {
        assert_eq!(keyword_coverage("", "anything"), 1.0);
    }

    fn weight_of(weights: &[RequirementWeight], skill: &str) -> f32 {
        weights
            .iter()
            .find(|requirement| requirement.skill == skill)
            .unwrap_or_else(|| panic!("no weight for {skill}"))
            .weight
    }

    #[test]
    fn weight_rises_with_frequency() {
        let job = make_job("docker docker docker rust", vec!["docker", "rust"]);
        let weights = weighted_requirements(&job);
        assert!(weight_of(&weights, "docker") > weight_of(&weights, "rust"));
        // sorted descending → the most frequent requirement leads.
        assert_eq!(weights[0].skill, "docker");
    }

    #[test]
    fn title_mention_boosts_weight() {
        let job = JobDescription {
            title: Some("Senior Rust Engineer".into()),
            raw_text: "rust python".into(),
            skills: vec!["rust".into(), "python".into()],
        };
        let weights = weighted_requirements(&job);
        // equal frequency, but rust is in the title → ranked first.
        assert!(weight_of(&weights, "rust") > weight_of(&weights, "python"));
    }

    #[test]
    fn required_cue_outweighs_nice_to_have() {
        let job = make_job(
            "Python is required. Docker is a nice-to-have bonus.",
            vec!["python", "docker"],
        );
        let weights = weighted_requirements(&job);
        assert!(weight_of(&weights, "python") > weight_of(&weights, "docker"));
    }
}
