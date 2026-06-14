pub mod ats;
pub mod diff;
pub mod matching;
pub mod pipeline;
pub mod skills;
pub mod text;
pub mod validation;

use serde::{Deserialize, Serialize};

pub use ats::AuditReport;
pub use pipeline::{Pipeline, PipelineOptions};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Cv {
    pub name: Option<String>,
    pub headline: Option<String>,
    pub summary: Option<String>,
    pub skills: Vec<String>,
    pub experience: Vec<Experience>,
    pub education: Vec<Education>,
    pub raw_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub company: Option<String>,
    pub role: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Education {
    pub institution: Option<String>,
    pub degree: Option<String>,
    pub year: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDescription {
    pub title: Option<String>,
    pub raw_text: String,
    pub skills: Vec<String>,
}
