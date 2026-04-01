#![allow(dead_code)]

//! OpenAI提供商实现

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use anyhow::{Result, Context};

use super::{AiProvider, ChatRequest, ChatResponse, Message, ProviderConfig, ProviderType, Role, TokenUsage};

pub struct OpenAiProvider {
    client: Client,
    config: ProviderConfig,
    base_url: String,
}

impl OpenAiProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self> {
        let base_url = config.api_base
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            base_url,
        })
    }

    fn messages_to_openai_format(&self, messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|m| json!({
                "role": m.role.as_str(),
                "content": m.content,
            }))
            .collect()
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = json!({
            "model": self.config.model,
            "messages": self.messages_to_openai_format(&request.messages),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": false,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "OpenAI API error ({}): {}",
                status,
                error_text
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = response_json["choices"][0]["finish_reason"]
            .as_str()
            .map(|s| s.to_string());

        let usage = response_json["usage"].as_object().map(|u| TokenUsage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatResponse {
            content,
            usage,
            finish_reason,
        })
    }

    async fn is_available(&self) -> bool {
        if self.config.api_key.is_empty() {
            return false;
        }

        // 尝试一个简单的请求来验证API key
        let test_request = ChatRequest {
            messages: vec![Message {
                role: Role::User,
                content: "Hi".to_string(),
            }],
            max_tokens: 5,
            temperature: 0.0,
            stream: false,
        };

        match self.chat(test_request).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn name(&self) -> &'static str {
        "OpenAI"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAi
    }
}
