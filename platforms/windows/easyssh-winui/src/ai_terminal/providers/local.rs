#![allow(dead_code)]

//! 本地模型提供商实现
//!
//! 支持：
//! - llama.cpp (HTTP server mode)
//! - Ollama
//! - llamafile

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::Duration;
use anyhow::{Result, Context};

use super::{AiProvider, ChatRequest, ChatResponse, LocalModelType, Message, ProviderConfig, ProviderType, Role, TokenUsage};

pub struct LocalProvider {
    client: Client,
    config: ProviderConfig,
    model_type: LocalModelType,
    base_url: String,
}

impl LocalProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        let model_type = ProviderConfig::default().local_model_type.unwrap_or(LocalModelType::Ollama);

        let base_url = match model_type {
            LocalModelType::LlamaCpp => "http://localhost:8080".to_string(),
            LocalModelType::Ollama => "http://localhost:11434".to_string(),
            LocalModelType::Llamafile => "http://localhost:8080".to_string(),
        };

        Self {
            client,
            config: ProviderConfig::local(),
            model_type,
            base_url,
        }
    }

    pub fn with_config(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        let model_type = config.local_model_type.unwrap_or(LocalModelType::Ollama);

        let base_url = config.api_base.clone().unwrap_or_else(|| match model_type {
            LocalModelType::LlamaCpp => "http://localhost:8080".to_string(),
            LocalModelType::Ollama => "http://localhost:11434".to_string(),
            LocalModelType::Llamafile => "http://localhost:8080".to_string(),
        });

        Self {
            client,
            config,
            model_type,
            base_url,
        }
    }

    fn format_messages_for_local(&self, messages: &[Message]) -> String {
        // 使用简单的聊天格式
        let mut formatted = String::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    formatted.push_str(&format!("System: {}\n", msg.content));
                }
                Role::User => {
                    formatted.push_str(&format!("User: {}\n", msg.content));
                }
                Role::Assistant => {
                    formatted.push_str(&format!("Assistant: {}\n", msg.content));
                }
            }
        }

        formatted.push_str("Assistant: ");
        formatted
    }

    async fn chat_with_ollama(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.base_url);

        // 构建消息历史
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| json!({
                "role": m.role.as_str(),
                "content": m.content,
            }))
            .collect();

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens,
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Ollama API error ({}): {}",
                status,
                error_text
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let content = response_json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let done_reason = response_json["done_reason"]
            .as_str()
            .map(|s| s.to_string());

        Ok(ChatResponse {
            content,
            usage: None, // Ollama may not provide usage stats
            finish_reason: done_reason,
        })
    }

    async fn chat_with_llamacpp(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/completion", self.base_url);

        // llama.cpp uses a simple prompt format
        let prompt = self.format_messages_for_local(&request.messages);

        let body = json!({
            "prompt": prompt,
            "temperature": request.temperature,
            "n_predict": request.max_tokens,
            "stop": ["\nUser:", "\nSystem:"],
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to connect to llama.cpp server")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "llama.cpp API error ({}): {}",
                status,
                error_text
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse llama.cpp response")?;

        let content = response_json["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_type = response_json["stop_type"]
            .as_str()
            .map(|s| s.to_string());

        let tokens_evaluated = response_json["tokens_evaluated"].as_u64().unwrap_or(0) as u32;
        let tokens_predicted = response_json["tokens_predicted"].as_u64().unwrap_or(0) as u32;

        Ok(ChatResponse {
            content: content.trim().to_string(),
            usage: Some(TokenUsage {
                prompt_tokens: tokens_evaluated,
                completion_tokens: tokens_predicted,
                total_tokens: tokens_evaluated + tokens_predicted,
            }),
            finish_reason: stop_type,
        })
    }

    /// 检查Ollama是否运行
    pub async fn is_ollama_running(&self) -> bool {
        match self.client.get(&format!("{}/api/tags", self.base_url)).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// 检查llama.cpp server是否运行
    pub async fn is_llamacpp_running(&self) -> bool {
        match self.client.get(&format!("{}/health", self.base_url)).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// 获取可用模型列表（Ollama）
    pub async fn list_models(&self) -> Result<Vec<String>> {
        if self.model_type != LocalModelType::Ollama {
            return Ok(vec![]);
        }

        let url = format!("{}/api/tags", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list Ollama models")?;

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse Ollama models response")?;

        let models: Vec<String> = response_json["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(models)
    }

    /// 启动本地模型（如果配置了路径）
    pub async fn start_local_model(&self) -> Result<()> {
        if let Some(ref path) = self.config.local_model_path {
            match self.model_type {
                LocalModelType::Llamafile => {
                    // llamafile是自包含的可执行文件
                    let _child = Command::new(path)
                        .arg("--server")
                        .arg("--nobrowser")
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .context("Failed to start llamafile")?;

                    // 等待服务启动
                    for _ in 0..30 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        if self.is_llamacpp_running().await {
                            return Ok(());
                        }
                    }

                    return Err(anyhow::anyhow!("llamafile failed to start within 30 seconds"));
                }
                _ => {
                    // Ollama和llama.cpp需要手动启动
                    return Err(anyhow::anyhow!("Please start the {} server manually", self.model_type.as_str()));
                }
            }
        }

        Ok(())
    }
}

impl LocalModelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama.cpp",
            Self::Ollama => "Ollama",
            Self::Llamafile => "llamafile",
        }
    }
}

#[async_trait]
impl AiProvider for LocalProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        match self.model_type {
            LocalModelType::Ollama => self.chat_with_ollama(request).await,
            LocalModelType::LlamaCpp | LocalModelType::Llamafile => {
                self.chat_with_llamacpp(request).await
            }
        }
    }

    async fn is_available(&self) -> bool {
        match self.model_type {
            LocalModelType::Ollama => self.is_ollama_running().await,
            LocalModelType::LlamaCpp | LocalModelType::Llamafile => {
                self.is_llamacpp_running().await
            }
        }
    }

    fn name(&self) -> &'static str {
        match self.model_type {
            LocalModelType::LlamaCpp => "llama.cpp",
            LocalModelType::Ollama => "Ollama",
            LocalModelType::Llamafile => "llamafile",
        }
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Local
    }
}
