//! LLM translation via OpenAI-compatible chat completions.
//! One implementation covers OpenAI, DeepSeek, Moonshot, SiliconFlow, etc.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{client, with_timeout};
use crate::providers::{Provider, ProviderKind};
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

/// Build the translation prompt. Kept free so tests need no HTTP.
fn build_prompt(source: &str, target: &str) -> String {
    let src = if source == "auto" { "the detected source language" } else { source };
    format!(
        "Translate the user's text from {src} to {target}. Output ONLY the translated text. \
         No explanations, no notes. Preserve formatting, code blocks and placeholders exactly."
    )
}

pub struct Llm {
    name: String,
    display: String,
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl Llm {
    pub fn new(
        name: &str,
        display: &str,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Self {
        let base_url = base_url
            .unwrap_or("https://api.openai.com/v1")
            .trim_end_matches('/')
            .to_string();
        Self {
            name: name.to_string(),
            display: display.to_string(),
            base_url,
            model: model.unwrap_or("gpt-4o-mini").to_string(),
            client: client(Duration::from_secs(60)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub fn openai(cfg: &crate::core::config::ProviderSettings) -> Self {
        Self::new("openai", "OpenAI", cfg.base_url.as_deref(), cfg.model.as_deref())
    }

    pub fn anthropic(cfg: &crate::core::config::ProviderSettings) -> Self {
        Self::new(
            "anthropic",
            "Anthropic Claude",
            cfg.base_url.as_deref().or(Some("https://api.anthropic.com/v1")),
            cfg.model.as_deref().or(Some("claude-sonnet-4-5")),
        )
    }

    pub fn deepseek(cfg: &crate::core::config::ProviderSettings) -> Self {
        Self::new(
            "deepseek",
            "DeepSeek",
            cfg.base_url.as_deref().or(Some("https://api.deepseek.com/v1")),
            cfg.model.as_deref().or(Some("deepseek-chat")),
        )
    }
}

#[async_trait]
impl Provider for Llm {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: self.name.clone(),
            display_name: self.display.clone(),
            kind: ProviderKind::Llm,
            priority: match self.name.as_str() {
                "openai" => 40,
                "anthropic" => 41,
                "deepseek" => 42,
                _ => 50,
            },
            needs_key: true,
            max_chars: 8000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("pay-as-you-go".into()),
            configured: true,
        }
    }

    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        cred: &Credential,
    ) -> Result<TranslateResponse> {
        let key = cred.get("api_key").unwrap_or_default();
        if key.is_empty() {
            return Err(Error::AuthFailed {
                provider: self.name.clone(),
                reason: "missing api_key".into(),
            });
        }

        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message { role: "system".into(), content: build_prompt(&req.source_lang, &req.target_lang) },
                Message { role: "user".into(), content: req.text.clone() },
            ],
            temperature: 0.2,
            max_tokens: 4096,
        };

        let started = Instant::now();
        let resp = with_timeout(self.client.post(&url), req.timeout_ms)
            .header("Authorization", format!("Bearer {key}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("{}: {e}", self.name)))?;

        let parsed: ChatResponse =
            crate::core::http::json(&self.name, resp).await?;

        let text = parsed
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: self.name.clone(),
                reason: "empty completion".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: req.source_lang.clone(),
            target_lang: req.target_lang.clone(),
            provider: self.name.clone(),
            engine: self.model.clone(),
            latency_ms: started.elapsed().as_millis() as u64,
            request_id: String::new(),
            chars: Some(req.text.chars().count()),
            warnings: Vec::new(),
        })
    }

    async fn probe(&self, cred: &Credential) -> Result<()> {
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: Some(10_000),
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        self.translate_once(&req, cred).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_names_both_languages() {
        let p = build_prompt("en", "zh");
        assert!(p.contains("en") && p.contains("zh") && p.contains("ONLY"));
    }

    #[test]
    fn prompt_handles_auto_source() {
        assert!(build_prompt("auto", "ja").contains("detected source language"));
    }

    #[tokio::test]
    async fn missing_key_is_auth_failed() {
        let p = Llm::openai(&Default::default());
        let cred = Credential {
            provider: "openai".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
        };
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        let err = p.translate_once(&req, &cred).await.unwrap_err();
        assert_eq!(err.code(), "AUTH_FAILED");
    }

    #[test]
    fn parses_openai_completion_shape() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"你好"}}]}"#;
        let parsed: ChatResponse = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.choices[0].message.content, "你好");
    }

    #[test]
    fn info_reports_llm_kind() {
        let p = Llm::openai(&Default::default());
        assert_eq!(p.info().kind, ProviderKind::Llm);
        assert_eq!(p.info().name, "openai");
    }
}
