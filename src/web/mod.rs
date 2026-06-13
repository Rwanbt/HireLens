use axum::{
    extract::Json,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::core::matching::ScoreReason;
use crate::core::{AuditReport, Pipeline, PipelineOptions};
use crate::llm::{LlmProviderKind, LlmRouter};

static UI_HTML: &str = include_str!("ui.html");

// ===== Public entry point =====

pub async fn serve(port: u16, open_browser: bool) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/api/audit", post(audit))
        .route("/api/adapt", post(adapt));

    // M1 — bind to loopback only: the local UI must not be reachable from the
    // network. A CV and job description are sensitive personal data.
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("HireLens is running at http://localhost:{port}");
    println!("Press Ctrl+C to stop.");

    if open_browser {
        let url = format!("http://localhost:{port}");
        // Best-effort browser open — ignore failure
        #[cfg(target_os = "windows")]
        let _ = tokio::process::Command::new("cmd")
            .args(["/c", "start", &url])
            .spawn();
        #[cfg(target_os = "macos")]
        let _ = tokio::process::Command::new("open").arg(&url).spawn();
        #[cfg(target_os = "linux")]
        let _ = tokio::process::Command::new("xdg-open").arg(&url).spawn();
    }

    axum::serve(listener, app).await?;
    Ok(())
}

// ===== Handlers =====

async fn index() -> Html<&'static str> {
    Html(UI_HTML)
}

async fn audit(Json(req): Json<ApiRequest>) -> impl IntoResponse {
    match do_audit(req).await {
        Ok(report) => (
            StatusCode::OK,
            Json(ApiResponse::ok(AuditData::from(report))),
        ),
        Err(e) => (
            StatusCode::OK,
            Json(ApiResponse::<AuditData>::err(friendly_error(&e))),
        ),
    }
}

async fn adapt(Json(req): Json<ApiRequest>) -> impl IntoResponse {
    match do_adapt(req).await {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::ok(data))),
        Err(e) => (
            StatusCode::OK,
            Json(ApiResponse::<AdaptData>::err(friendly_error(&e))),
        ),
    }
}

// ===== Business logic =====

async fn do_audit(req: ApiRequest) -> anyhow::Result<AuditReport> {
    let cv = req.cv.trim();
    let job = req.job.trim();
    anyhow::ensure!(!cv.is_empty(), "Le CV ne peut pas être vide.");
    anyhow::ensure!(!job.is_empty(), "L'offre d'emploi ne peut pas être vide.");

    let (kind, offline) = parse_provider(&req.provider);
    let router = LlmRouter::new(kind)?;
    let pipeline = Pipeline::new(router);
    pipeline
        .audit_text(
            cv,
            job,
            PipelineOptions {
                offline,
                use_cache: true,
            },
        )
        .await
}

async fn do_adapt(req: ApiRequest) -> anyhow::Result<AdaptData> {
    let cv = req.cv.trim();
    let job = req.job.trim();
    anyhow::ensure!(!cv.is_empty(), "Le CV ne peut pas être vide.");
    anyhow::ensure!(!job.is_empty(), "L'offre d'emploi ne peut pas être vide.");

    let (kind, offline) = parse_provider(&req.provider);
    let router = LlmRouter::new(kind)?;
    let pipeline = Pipeline::new(router);
    let result = pipeline
        .adapt_text(
            cv,
            job,
            PipelineOptions {
                offline,
                use_cache: true,
            },
        )
        .await?;

    Ok(AdaptData {
        markdown: result.rendered_markdown,
        audit: AuditData::from(result.audit),
    })
}

// ===== Helpers =====

fn parse_provider(provider: &str) -> (LlmProviderKind, bool) {
    match provider {
        "openai" => (LlmProviderKind::OpenAi, false),
        "ollama" => (LlmProviderKind::Ollama, false),
        "lmstudio" => (LlmProviderKind::LmStudio, false),
        _ => (LlmProviderKind::Ollama, true), // "offline" and anything unknown
    }
}

fn friendly_error(err: &anyhow::Error) -> String {
    // M2 — never leak internal error detail (cause chain, file paths, parser
    // messages) to the HTTP client. Log the full chain server-side and return
    // only a curated message. Known patterns map to actionable hints; anything
    // else falls back to a generic message.
    tracing::error!("{err:?}");

    let msg = err.to_string();
    if msg.contains("10061") || msg.contains("Connection refused") || msg.contains("tcp connect") {
        if msg.contains("11434") {
            return "Ollama n'est pas lancé. Démarrez-le avec : ollama serve".to_owned();
        }
        if msg.contains("1234") {
            return "LM Studio n'est pas lancé. Ouvrez LM Studio et démarrez le serveur local."
                .to_owned();
        }
    }
    if msg.contains("OPENAI_API_KEY") || msg.contains("401") || msg.contains("Unauthorized") {
        return "Clé API OpenAI manquante ou invalide. Définissez la variable OPENAI_API_KEY."
            .to_owned();
    }
    // User-facing validation messages are intentional (not internal leaks).
    if msg.contains("vide") || msg.contains("empty") {
        return msg;
    }
    "Une erreur interne est survenue. Consultez les logs du serveur pour le détail.".to_owned()
}

// ===== Request / Response types =====

#[derive(Debug, Deserialize)]
struct ApiRequest {
    cv: String,
    job: String,
    #[serde(default = "default_provider")]
    provider: String,
}

fn default_provider() -> String {
    "offline".to_owned()
}

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }
    fn err(message: String) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(message),
        }
    }
}

#[derive(Debug, Serialize)]
struct AuditData {
    score: u8,
    skill_match_ratio: f32,
    cv_skills: Vec<String>,
    job_skills: Vec<String>,
    matched_skills: Vec<String>,
    missing_skills: Vec<String>,
    explanations: Vec<ScoreReason>,
}

impl From<AuditReport> for AuditData {
    fn from(r: AuditReport) -> Self {
        Self {
            score: r.score.score,
            skill_match_ratio: r.score.skill_match_ratio,
            cv_skills: r.cv_skills,
            job_skills: r.job_skills,
            matched_skills: r.matched_skills,
            missing_skills: r.missing_skills,
            explanations: r.explanations,
        }
    }
}

#[derive(Debug, Serialize)]
struct AdaptData {
    markdown: String,
    audit: AuditData,
}
