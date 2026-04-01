#![allow(dead_code)]

//! AI提供商抽象和实现
//!
//! 支持：
//! - OpenAI API (GPT-4, GPT-3.5)
//! - Anthropic Claude API
//! - 本地模型 (llama.cpp, Ollama)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

pub mod openai;
pub mod claude;
pub mod local;


/// 提供商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    OpenAi,
    Claude,
    Local,
}

/// 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: String,
    pub api_base: Option<String>,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_secs: u64,
    /// 本地模型路径
    pub local_model_path: Option<String>,
    /// 本地模型类型
    pub local_model_type: Option<LocalModelType>,
    /// 额外参数
    pub extra_params: HashMap<String, String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::Claude,
            api_key: String::new(),
            api_base: None,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 2048,
            temperature: 0.3,
            timeout_secs: 30,
            local_model_path: None,
            local_model_type: None,
            extra_params: HashMap::new(),
        }
    }
}

impl ProviderConfig {
    /// 创建OpenAI配置
    pub fn openai(api_key: &str) -> Self {
        Self {
            provider_type: ProviderType::OpenAi,
            api_key: api_key.to_string(),
            api_base: Some("https://api.openai.com/v1".to_string()),
            model: "gpt-4".to_string(),
            max_tokens: 2048,
            temperature: 0.3,
            timeout_secs: 30,
            local_model_path: None,
            local_model_type: None,
            extra_params: HashMap::new(),
        }
    }

    /// 创建Claude配置
    pub fn claude(api_key: &str) -> Self {
        Self {
            provider_type: ProviderType::Claude,
            api_key: api_key.to_string(),
            api_base: Some("https://api.anthropic.com".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 2048,
            temperature: 0.3,
            timeout_secs: 30,
            local_model_path: None,
            local_model_type: None,
            extra_params: HashMap::new(),
        }
    }

    /// 创建本地模型配置
    pub fn local() -> Self {
        Self {
            provider_type: ProviderType::Local,
            api_key: String::new(),
            api_base: None,
            model: "llama-3-8b-instruct".to_string(),
            max_tokens: 2048,
            temperature: 0.3,
            timeout_secs: 60,
            local_model_path: None,
            local_model_type: Some(LocalModelType::Ollama),
            extra_params: HashMap::new(),
        }
    }
}

/// 本地模型类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelType {
    LlamaCpp,
    Ollama,
    Llamafile,
}

/// AI消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// 聊天完成请求
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stream: bool,
}

/// 聊天完成响应
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
}

/// Token使用统计
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// AI提供商trait
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// 发送聊天请求
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// 检查提供商是否可用
    async fn is_available(&self) -> bool;

    /// 获取提供商名称
    fn name(&self) -> &'static str;

    /// 获取提供商类型
    fn provider_type(&self) -> ProviderType;
}

/// 流式响应处理
#[async_trait]
pub trait StreamingHandler: Send {
    async fn on_chunk(&mut self, chunk: &str);
    async fn on_complete(&mut self, response: ChatResponse);
    async fn on_error(&mut self, error: &str);
}

/// Mock provider for testing
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AiProvider for MockProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
        Ok(ChatResponse {
            content: "Mock response".to_string(),
            usage: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "MockProvider"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Local
    }
}
