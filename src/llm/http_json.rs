use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::llm::{AdaptationRequest, ExtractSkillsRequest};

const SYSTEM_PROMPT: &str = r#"You are an ATS and CV analysis assistant.
Return strict JSON only. Do not include markdown, prose, or explanations.
Never add skills, education, certifications, employers, dates, or experience not provided in the input.
For adaptation, only rewrite existing bullet meaning and only use allowed_skills."#;

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

pub async fn post_openai_compatible<T>(
    client: &Client,
    url: &str,
    bearer: Option<&str>,
    model: &str,
    user_prompt: String,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let payload = OpenAiChatRequest {
        model: model.to_owned(),
        messages: vec![
            ChatMessage {
                role: "system".to_owned(),
                content: SYSTEM_PROMPT.to_owned(),
            },
            ChatMessage {
                role: "user".to_owned(),
                content: user_prompt,
            },
        ],
        temperature: 0.0,
        response_format: ResponseFormat {
            kind: "json_object".to_owned(),
        },
    };

    let mut request = client.post(url).json(&payload);
    if let Some(token) = bearer {
        request = request.bearer_auth(token);
    }

    let response = request.send().await?.error_for_status()?;
    let chat: ChatResponse = response.json().await?;
    let content = chat
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("LLM response had no choices"))?
        .message
        .content;

    parse_json_content(&content)
}

pub fn extract_skills_prompt(request: &ExtractSkillsRequest) -> Result<String> {
    Ok(format!(
        "Extract skill names from this {}. Return JSON matching {{\"skills\":[\"skill\"]}}.\n\n{}",
        request.source_name, request.text
    ))
}

pub fn adaptation_prompt(request: &AdaptationRequest) -> Result<String> {
    Ok(format!(
        "Create a structured adaptation plan. Return JSON matching {{\"prioritized_skills\":[],\"selected_bullets\":[{{\"experience_id\":\"id\",\"bullet\":\"exact original bullet\"}}]}}.\nOnly select exact skills and exact bullets from the CV JSON. Do not rewrite text.\nAllowed skills: {}\nCV JSON: {}\nJob JSON: {}",
        serde_json::to_string(&request.allowed_skills)?,
        serde_json::to_string(&request.cv)?,
        serde_json::to_string(&request.job)?,
    ))
}

pub fn parse_json_content<T>(content: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(content).with_context(|| "LLM returned invalid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::AdaptationResponse;

    #[test]
    fn rejects_unknown_fields_in_adaptation_json() {
        let content = r#"{
            "headline": "Invented headline",
            "prioritized_skills": ["rust"],
            "selected_bullets": []
        }"#;

        let error = parse_json_content::<AdaptationResponse>(content)
            .expect_err("unknown fields must be rejected");

        assert!(error.to_string().contains("LLM returned invalid JSON"));
    }

    #[test]
    fn accepts_strict_adaptation_json() {
        let content = r#"{
            "prioritized_skills": ["rust"],
            "selected_bullets": [
                {"experience_id": "exp-1", "bullet": "Built Rust services."}
            ]
        }"#;

        let parsed =
            parse_json_content::<AdaptationResponse>(content).expect("strict JSON should parse");

        assert_eq!(parsed.prioritized_skills, vec!["rust"]);
        assert_eq!(parsed.selected_bullets[0].experience_id, "exp-1");
    }
}
