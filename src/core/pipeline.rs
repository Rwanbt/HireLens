use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::core::ats::{compute_audit, merge_skills};
use crate::core::validation::{diff_markdown, validate_adaptation};
use crate::core::{AuditReport, JobDescription};
use crate::export::render_cv;
use crate::llm::{AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, LlmRouter};
use crate::parser::{parse_cv_file, parse_cv_markdown, parse_job_file, parse_job_text};
use crate::utils::cache::Cache;

#[derive(Clone)]
pub struct Pipeline {
    llm: LlmRouter,
}

#[derive(Debug, Clone, Copy)]
pub struct PipelineOptions {
    pub offline: bool,
    pub use_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptedCv {
    pub rendered_markdown: String,
    pub audit: AuditReport,
    pub validation_diff: String,
}

impl Pipeline {
    pub fn new(llm: LlmRouter) -> Self {
        Self { llm }
    }

    pub async fn audit(
        &self,
        cv_path: &Path,
        job_path: &Path,
        options: PipelineOptions,
    ) -> Result<AuditReport> {
        let mut cv = parse_cv_file(cv_path)?;
        let mut job = parse_job_file(job_path)?;
        self.enrich_skills(&mut cv.skills, &mut job, cv_path, job_path, options)
            .await?;
        Ok(compute_audit(&cv, &job))
    }

    pub async fn adapt(
        &self,
        cv_path: &Path,
        job_path: &Path,
        options: PipelineOptions,
    ) -> Result<AdaptedCv> {
        let mut cv = parse_cv_file(cv_path)?;
        let mut job = parse_job_file(job_path)?;

        // C1 — Anti-hallucination ground truth: snapshot the skills declared in
        // the original CV BEFORE enrichment. enrich_skills() merges LLM-extracted
        // skills into cv.skills; without this snapshot the LLM could widen the
        // whitelist that validate_adaptation() enforces and smuggle invented
        // skills into the output. See ADR-0001.
        let allowed_skills = cv.skills.clone();

        self.enrich_skills(&mut cv.skills, &mut job, cv_path, job_path, options)
            .await?;
        let audit = compute_audit(&cv, &job);

        // Drop the enriched skills before adaptation/validation/render: only
        // skills present in the original CV may reach the output.
        cv.skills = allowed_skills.clone();
        let adaptation = self
            .adaptation(&cv, &job, allowed_skills, cv_path, job_path, options)
            .await?;
        validate_adaptation(&cv, &adaptation)?;
        let rendered_markdown = render_cv(&cv, Some(&adaptation));
        let diff = diff_markdown(&cv.raw_markdown, &rendered_markdown);

        Ok(AdaptedCv {
            rendered_markdown,
            audit,
            validation_diff: diff,
        })
    }

    /// Audit directly from text strings — used by the web UI (no temp files needed).
    pub async fn audit_text(
        &self,
        cv_text: &str,
        job_text: &str,
        options: PipelineOptions,
    ) -> Result<AuditReport> {
        let mut cv = parse_cv_markdown(cv_text)?;
        let mut job = parse_job_text(job_text);
        self.enrich_skills_text(&mut cv.skills, &mut job, cv_text, job_text, options)
            .await?;
        Ok(compute_audit(&cv, &job))
    }

    /// Adapt directly from text strings — used by the web UI (no temp files needed).
    pub async fn adapt_text(
        &self,
        cv_text: &str,
        job_text: &str,
        options: PipelineOptions,
    ) -> Result<AdaptedCv> {
        let mut cv = parse_cv_markdown(cv_text)?;
        let mut job = parse_job_text(job_text);

        // C1 — Anti-hallucination ground truth: snapshot original CV skills
        // BEFORE enrichment so the LLM cannot widen the validation whitelist.
        // See ADR-0001 and the matching guard in `adapt()`.
        let allowed_skills = cv.skills.clone();

        self.enrich_skills_text(&mut cv.skills, &mut job, cv_text, job_text, options)
            .await?;
        let audit = compute_audit(&cv, &job);

        // Drop the enriched skills before adaptation/validation/render.
        cv.skills = allowed_skills.clone();
        let adaptation = self
            .adaptation_text(&cv, &job, allowed_skills, cv_text, job_text, options)
            .await?;
        validate_adaptation(&cv, &adaptation)?;
        let rendered_markdown = render_cv(&cv, Some(&adaptation));
        let diff = diff_markdown(&cv.raw_markdown, &rendered_markdown);

        Ok(AdaptedCv {
            rendered_markdown,
            audit,
            validation_diff: diff,
        })
    }

    async fn enrich_skills_text(
        &self,
        cv_skills: &mut Vec<String>,
        job: &mut JobDescription,
        cv_text: &str,
        job_text: &str,
        options: PipelineOptions,
    ) -> Result<()> {
        let cache = Cache::configured();
        let provider = self.llm.provider_name();
        let cv_key = cache.key("extract_cv_web", &[], cv_text, provider)?;
        let job_key = cache.key("extract_job_web", &[], job_text, provider)?;

        let extracted_cv = if options.offline {
            crate::llm::offline_extract_skills(ExtractSkillsRequest {
                source_name: "CV".to_owned(),
                text: cv_text.to_owned(),
            })
            .await?
        } else if options.use_cache {
            cache
                .get_or_insert_json(&cv_key, || async {
                    self.llm
                        .extract_skills(ExtractSkillsRequest {
                            source_name: "CV".to_owned(),
                            text: cv_text.to_owned(),
                        })
                        .await
                })
                .await?
        } else {
            self.llm
                .extract_skills(ExtractSkillsRequest {
                    source_name: "CV".to_owned(),
                    text: cv_text.to_owned(),
                })
                .await?
        };

        let extracted_job = if options.offline {
            crate::llm::offline_extract_skills(ExtractSkillsRequest {
                source_name: "job description".to_owned(),
                text: job_text.to_owned(),
            })
            .await?
        } else if options.use_cache {
            cache
                .get_or_insert_json(&job_key, || async {
                    self.llm
                        .extract_skills(ExtractSkillsRequest {
                            source_name: "job description".to_owned(),
                            text: job_text.to_owned(),
                        })
                        .await
                })
                .await?
        } else {
            self.llm
                .extract_skills(ExtractSkillsRequest {
                    source_name: "job description".to_owned(),
                    text: job_text.to_owned(),
                })
                .await?
        };

        *cv_skills = merge_skills(cv_skills, &extracted_cv.skills);
        job.skills = merge_skills(&job.skills, &extracted_job.skills);
        Ok(())
    }

    async fn adaptation_text(
        &self,
        cv: &crate::core::Cv,
        job: &JobDescription,
        allowed_skills: Vec<String>,
        cv_text: &str,
        job_text: &str,
        options: PipelineOptions,
    ) -> Result<AdaptationResponse> {
        let cache = Cache::configured();
        let body = serde_json::to_string(&(cv, job, &allowed_skills))?;
        let combined = format!("{cv_text}{job_text}");
        let key = cache.key("adapt_web", &[], &format!("{body}{combined}"), self.llm.provider_name())?;
        let request = AdaptationRequest {
            cv: cv.clone(),
            job: job.clone(),
            allowed_skills,
        };

        if options.offline {
            crate::llm::offline_adaptation(request).await
        } else if options.use_cache {
            cache
                .get_or_insert_json(&key, || async {
                    self.llm.generate_adaptation(request.clone()).await
                })
                .await
        } else {
            self.llm.generate_adaptation(request).await
        }
    }

    async fn enrich_skills(
        &self,
        cv_skills: &mut Vec<String>,
        job: &mut JobDescription,
        cv_path: &Path,
        job_path: &Path,
        options: PipelineOptions,
    ) -> Result<()> {
        let cv_text = std::fs::read_to_string(cv_path)?;
        let cache = Cache::configured();
        let provider = self.llm.provider_name();
        let cv_key = cache.key("extract_cv", &[cv_path, job_path], &cv_text, provider)?;
        let job_key = cache.key("extract_job", &[cv_path, job_path], &job.raw_text, provider)?;

        let extracted_cv = if options.offline {
            crate::llm::offline_extract_skills(ExtractSkillsRequest {
                source_name: "CV".to_owned(),
                text: cv_text,
            })
            .await?
        } else if options.use_cache {
            cache
                .get_or_insert_json(&cv_key, || async {
                    self.llm
                        .extract_skills(ExtractSkillsRequest {
                            source_name: "CV".to_owned(),
                            text: cv_text,
                        })
                        .await
                })
                .await?
        } else {
            self.llm
                .extract_skills(ExtractSkillsRequest {
                    source_name: "CV".to_owned(),
                    text: cv_text,
                })
                .await?
        };

        let extracted_job = if options.offline {
            crate::llm::offline_extract_skills(ExtractSkillsRequest {
                source_name: "job description".to_owned(),
                text: job.raw_text.clone(),
            })
            .await?
        } else if options.use_cache {
            cache
                .get_or_insert_json(&job_key, || async {
                    self.llm
                        .extract_skills(ExtractSkillsRequest {
                            source_name: "job description".to_owned(),
                            text: job.raw_text.clone(),
                        })
                        .await
                })
                .await?
        } else {
            self.llm
                .extract_skills(ExtractSkillsRequest {
                    source_name: "job description".to_owned(),
                    text: job.raw_text.clone(),
                })
                .await?
        };

        *cv_skills = merge_skills(cv_skills, &extracted_cv.skills);
        job.skills = merge_skills(&job.skills, &extracted_job.skills);
        Ok(())
    }

    async fn adaptation(
        &self,
        cv: &crate::core::Cv,
        job: &JobDescription,
        allowed_skills: Vec<String>,
        cv_path: &Path,
        job_path: &Path,
        options: PipelineOptions,
    ) -> Result<AdaptationResponse> {
        let cache = Cache::configured();
        let body = serde_json::to_string(&(cv, job, &allowed_skills))?;
        let key = cache.key("adapt", &[cv_path, job_path], &body, self.llm.provider_name())?;
        let request = AdaptationRequest {
            cv: cv.clone(),
            job: job.clone(),
            allowed_skills,
        };

        if options.offline {
            crate::llm::offline_adaptation(request).await
        } else if options.use_cache {
            cache
                .get_or_insert_json(&key, || async {
                    self.llm.generate_adaptation(request.clone()).await
                })
                .await
        } else {
            self.llm.generate_adaptation(request).await
        }
    }
}
