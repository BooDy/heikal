use crate::models::Article;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

// Anthropic types
#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

// Gemini types
#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiCandidatePart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiCandidatePart>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

pub async fn summarize_unread(
    provider: &str,
    api_token: &str,
    model: &str,
    api_base: &str,
    articles: &[Article],
) -> Result<String, String> {
    if articles.is_empty() {
        return Ok("No unread articles in this feed.".to_string());
    }

    let mut prompt = String::from(
        "Please summarize the following unread RSS feed articles in a concise bullet-point summary. Group by topic if appropriate, and keep it brief:\n\n"
    );
    for (i, art) in articles.iter().enumerate() {
        prompt.push_str(&format!("{}. {}\n", i + 1, art.title));
        if let Some(desc) = &art.description {
            let desc_clean = if desc.len() > 300 {
                format!("{}...", &desc[..300])
            } else {
                desc.clone()
            };
            prompt.push_str(&format!("   Description: {}\n", desc_clean));
        }
    }

    let client = reqwest::Client::new();

    match provider.to_lowercase().as_str() {
        "openai" | "openrouter" | "ollama" | "custom" => {
            let url = if !api_base.is_empty() {
                if api_base.ends_with("/chat/completions") {
                    api_base.to_string()
                } else if api_base.ends_with('/') {
                    format!("{}chat/completions", api_base)
                } else {
                    format!("{}/chat/completions", api_base)
                }
            } else if provider.to_lowercase() == "openrouter" {
                "https://openrouter.ai/api/v1/chat/completions".to_string()
            } else {
                "https://api.openai.com/v1/chat/completions".to_string()
            };

            let req_body = OpenAiRequest {
                model: model.to_string(),
                messages: vec![
                    OpenAiMessage {
                        role: "system".to_string(),
                        content: "You are a helpful assistant that summarizes RSS feed articles.".to_string(),
                    },
                    OpenAiMessage {
                        role: "user".to_string(),
                        content: prompt,
                    },
                ],
            };

            let mut req = client.post(&url).json(&req_body);
            if !api_token.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", api_token));
            }
            
            if provider.to_lowercase() == "openrouter" {
                req = req.header("HTTP-Referer", "https://github.com/BooDy/heikal");
                req = req.header("X-Title", "Heikal TUI");
            }

            let resp = req.send().await.map_err(|e| format!("HTTP request failed: {}", e))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let err_text = resp.text().await.unwrap_or_default();
                return Err(format!("API returned error status {}: {}", status, err_text));
            }

            let res_body: OpenAiResponse = resp.json().await.map_err(|e| format!("Failed to parse response JSON: {}", e))?;
            if let Some(choice) = res_body.choices.first() {
                Ok(choice.message.content.clone())
            } else {
                Err("No summary returned from model".to_string())
            }
        }
        "anthropic" => {
            let url = if !api_base.is_empty() {
                api_base.to_string()
            } else {
                "https://api.anthropic.com/v1/messages".to_string()
            };

            let req_body = AnthropicRequest {
                model: model.to_string(),
                max_tokens: 1024,
                messages: vec![AnthropicMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
            };

            let resp = client
                .post(&url)
                .header("x-api-key", api_token)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&req_body)
                .send()
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err_text = resp.text().await.unwrap_or_default();
                return Err(format!("API returned error status {}: {}", status, err_text));
            }

            let res_body: AnthropicResponse = resp.json().await.map_err(|e| format!("Failed to parse response JSON: {}", e))?;
            if let Some(content_block) = res_body.content.first() {
                Ok(content_block.text.clone())
            } else {
                Err("No summary returned from model".to_string())
            }
        }
        "gemini" => {
            let url = if !api_base.is_empty() {
                api_base.to_string()
            } else {
                format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    model, api_token
                )
            };

            let req_body = GeminiRequest {
                contents: vec![GeminiContent {
                    parts: vec![GeminiPart { text: prompt }],
                }],
            };

            let resp = client
                .post(&url)
                .header("content-type", "application/json")
                .json(&req_body)
                .send()
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err_text = resp.text().await.unwrap_or_default();
                return Err(format!("API returned error status {}: {}", status, err_text));
            }

            let res_body: GeminiResponse = resp.json().await.map_err(|e| format!("Failed to parse response JSON: {}", e))?;
            if let Some(candidate) = res_body.candidates.first() {
                if let Some(part) = candidate.content.parts.first() {
                    Ok(part.text.clone())
                } else {
                    Err("No content parts returned from Gemini".to_string())
                }
            } else {
                Err("No summary candidates returned from Gemini".to_string())
            }
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}
