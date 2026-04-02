#![allow(dead_code)]

//! Claude提供商实现

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use super::{
    AiProvider, ChatRequest, ChatResponse, Message, ProviderConfig, ProviderType, Role, TokenUsage,
};

pub struct ClaudeProvider {
    client: Client,
    config: ProviderConfig,
    base_url: String,
}

impl ClaudeProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self> {
        let base_url = config
            .api_base
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());

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

    fn messages_to_claude_format(&self, messages: &[Message]) -> (Option<String>, Vec<Value>) {
        // 分离系统提示和用户/助手消息
        let mut system_prompt = None;
        let mut claude_messages = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    system_prompt = Some(msg.content.clone());
                }
                Role::User => {
                    claude_messages.push(json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                }
                Role::Assistant => {
                    claude_messages.push(json!({
                        "role": "assistant",
                        "content": msg.content,
                    }));
                }
            }
        }

        (system_prompt, claude_messages)
    }
}

#[async_trait]
impl AiProvider for ClaudeProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/v1/messages", self.base_url);

        let (system_prompt, messages) = self.messages_to_claude_format(&request.messages);

        let mut body = json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        if let Some(system) = system_prompt {
            body["system"] = json!(system);
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Claude API error ({}): {}",
                status,
                error_text
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse Claude response")?;

        let content = response_json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = response_json["stop_reason"].as_str().map(|s| s.to_string());

        let usage = response_json["usage"].as_object().map(|u| TokenUsage {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32
                + u["output_tokens"].as_u64().unwrap_or(0) as u32,
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

        (self.chat(test_request).await).is_ok()
    }

    fn name(&self) -> &'static str {
        "Claude"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Claude
    }
}
