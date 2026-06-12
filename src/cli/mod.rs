use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

use crate::core::{AuditReport, Pipeline, PipelineOptions};
use crate::export::{MarkdownExporter, PdfExporter};
use crate::llm::{LlmProviderKind, LlmRouter};
use crate::utils::config::Config;

#[derive(Debug, Parser)]
#[command(name = "hirelens")]
#[command(about = "Hybrid AI-powered CV optimization engine")]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Score a CV against a job description without generating an adapted CV.
    Audit(AuditArgs),
    /// Create an optimized CV from trusted CV facts and a job description.
    Adapt(AdaptArgs),
    /// Render a CV to Markdown and optionally PDF.
    Build(BuildArgs),
    /// Open the graphical user interface.
    Gui(GuiArgs),
}

#[derive(Debug, ClapArgs)]
struct AuditArgs {
    cv: PathBuf,
    job: PathBuf,
    #[arg(long, value_enum)]
    provider: Option<ProviderArg>,
    /// Emit machine-readable JSON instead of a human report.
    #[arg(long)]
    json: bool,
    /// Fail when the ATS score is below this value.
    #[arg(long)]
    min_score: Option<u8>,
    #[arg(long)]
    offline: bool,
    #[arg(long)]
    no_cache: bool,
}

#[derive(Debug, ClapArgs)]
struct AdaptArgs {
    cv: PathBuf,
    job: PathBuf,
    #[arg(long, value_enum)]
    provider: Option<ProviderArg>,
    #[arg(long, default_value = "optimized-cv.md")]
    output: PathBuf,
    /// Emit machine-readable JSON summary instead of a human report.
    #[arg(long)]
    json: bool,
    /// Print the diff between the original CV markdown and rendered output.
    #[arg(long)]
    diff: bool,
    /// Fail without writing output when the ATS score is below this value.
    #[arg(long)]
    min_score: Option<u8>,
    #[arg(long)]
    offline: bool,
    #[arg(long)]
    no_cache: bool,
}

#[derive(Debug, ClapArgs)]
struct BuildArgs {
    cv: PathBuf,
    #[arg(long, default_value = "cv.md")]
    output: PathBuf,
    #[arg(long)]
    pdf: bool,
}

#[derive(Debug, ClapArgs)]
pub struct GuiArgs {}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ProviderArg {
    Openai,
    Ollama,
    Lmstudio,
}

impl From<ProviderArg> for LlmProviderKind {
    fn from(value: ProviderArg) -> Self {
        match value {
            ProviderArg::Openai => Self::OpenAi,
            ProviderArg::Ollama => Self::Ollama,
            ProviderArg::Lmstudio => Self::LmStudio,
        }
    }
}

impl Args {
    pub fn is_gui(&self) -> bool {
        matches!(self.command, Command::Gui(_))
    }
}

pub async fn run(args: Args) -> Result<()> {
    let config = Config::load()?;
    match args.command {
        Command::Audit(args) => {
            let router = LlmRouter::new(resolve_provider(args.provider, &config)?)?;
            let pipeline = Pipeline::new(router);
            let report = pipeline
                .audit(
                    &args.cv,
                    &args.job,
                    PipelineOptions {
                        offline: args.offline || config.offline.unwrap_or(false),
                        use_cache: !args.no_cache && config.cache_enabled(),
                    },
                )
                .await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", format_audit_report(&report));
            }
            enforce_min_score(report.score.score, args.min_score)?;
        }
        Command::Adapt(args) => {
            let router = LlmRouter::new(resolve_provider(args.provider, &config)?)?;
            let pipeline = Pipeline::new(router);
            let adapted = pipeline
                .adapt(
                    &args.cv,
                    &args.job,
                    PipelineOptions {
                        offline: args.offline || config.offline.unwrap_or(false),
                        use_cache: !args.no_cache && config.cache_enabled(),
                    },
                )
                .await?;

            enforce_min_score(adapted.audit.score.score, args.min_score)?;
            MarkdownExporter::write(&adapted.rendered_markdown, &args.output)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&adapted)?);
            } else {
                println!(
                    "Adaptation complete\n\nOutput: {}\nATS score: {}/100\nMatched skills: {}\nMissing skills: {}",
                    args.output.display(),
                    adapted.audit.score.score,
                    format_list(&adapted.audit.matched_skills),
                    format_list(&adapted.audit.missing_skills),
                );
                if args.diff {
                    println!("\nDiff\n\n{}", adapted.validation_diff);
                }
            }
        }
        Command::Build(args) => {
            let cv = crate::parser::parse_cv_file(&args.cv)?;
            let rendered = crate::export::render_cv(&cv, None);
            MarkdownExporter::write(&rendered, &args.output)?;
            eprintln!("Wrote {}", args.output.display());

            if args.pdf {
                let pdf_path = args.output.with_extension("pdf");
                PdfExporter::write_with_pandoc(&args.output, &pdf_path).await?;
                eprintln!("Wrote {}", pdf_path.display());
            }
        }
        // GUI is handled before the tokio runtime in main.rs
        Command::Gui(_) => unreachable!("GUI command handled in main"),
    }

    Ok(())
}

fn resolve_provider(provider: Option<ProviderArg>, config: &Config) -> Result<LlmProviderKind> {
    if let Some(provider) = provider {
        return Ok(provider.into());
    }

    if let Some(provider) = &config.provider {
        return LlmProviderKind::parse(provider)
            .ok_or_else(|| anyhow::anyhow!("unknown provider in config: {provider}"));
    }

    Ok(LlmProviderKind::Ollama)
}

fn enforce_min_score(score: u8, min_score: Option<u8>) -> Result<()> {
    if let Some(min_score) = min_score {
        if score < min_score {
            bail!("ATS score {score}/100 is below required minimum {min_score}/100");
        }
    }
    Ok(())
}

fn format_audit_report(report: &AuditReport) -> String {
    format!(
        "ATS audit\n\nScore: {}/100\nSkill match: {:.0}%\n\nMatched skills: {}\nMissing skills: {}\nCV skills: {}\nJob skills: {}",
        report.score.score,
        report.score.skill_match_ratio * 100.0,
        format_list(&report.matched_skills),
        format_list(&report.missing_skills),
        format_list(&report.cv_skills),
        format_list(&report.job_skills),
    )
}

fn format_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ats::AtsScore;

    fn report() -> AuditReport {
        AuditReport {
            score: AtsScore {
                skill_match_ratio: 0.5,
                score: 50,
            },
            cv_skills: vec!["docker".into(), "rust".into()],
            job_skills: vec!["kubernetes".into(), "rust".into()],
            matched_skills: vec!["rust".into()],
            missing_skills: vec!["kubernetes".into()],
        }
    }

    #[test]
    fn formats_human_audit_report() {
        let output = format_audit_report(&report());

        assert!(output.contains("ATS audit"));
        assert!(output.contains("Score: 50/100"));
        assert!(output.contains("Matched skills: rust"));
        assert!(output.contains("Missing skills: kubernetes"));
    }

    #[test]
    fn min_score_rejects_low_score() {
        let error = enforce_min_score(49, Some(50)).expect_err("score should fail");

        assert!(error.to_string().contains("below required minimum 50/100"));
    }

    #[test]
    fn min_score_accepts_equal_score() {
        enforce_min_score(50, Some(50)).expect("equal score should pass");
    }

    #[test]
    fn resolves_provider_from_config() {
        let config = Config {
            provider: Some("lmstudio".into()),
            ..Config::default()
        };

        let provider = resolve_provider(None, &config).expect("provider should resolve");

        assert!(matches!(provider, LlmProviderKind::LmStudio));
    }

    #[test]
    fn provider_flag_overrides_config() {
        let config = Config {
            provider: Some("openai".into()),
            ..Config::default()
        };

        let provider =
            resolve_provider(Some(ProviderArg::Ollama), &config).expect("provider should resolve");

        assert!(matches!(provider, LlmProviderKind::Ollama));
    }
}
