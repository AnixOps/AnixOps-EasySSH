use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// HTTP methods supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Connect,
    Trace,
}

impl Default for HttpMethod {
    fn default() -> Self {
        HttpMethod::Get
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Connect => write!(f, "CONNECT"),
            HttpMethod::Trace => write!(f, "TRACE"),
        }
    }
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Auth {
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
    ApiKey {
        key: String,
        value: String,
        #[serde(default = "default_api_key_in")]
        in_: String, // header or query
    },
    Oauth2 {
        access_token: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        refresh_token: Option<String>,
    },
    Digest {
        username: String,
        password: String,
    },
}

fn default_api_key_in() -> String {
    "header".to_string()
}

impl Default for Auth {
    fn default() -> Self {
        Auth::None
    }
}

/// Body content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Body {
    None,
    Text {
        content: String,
    },
    Json {
        content: String,
    },
    Xml {
        content: String,
    },
    Form {
        #[serde(with = "serde_urlencoded_map")]
        data: HashMap<String, String>,
    },
    Multipart {
        parts: Vec<MultipartPart>,
    },
    Binary {
        #[serde(with = "base64_string")]
        data: Vec<u8>,
        filename: Option<String>,
        mime_type: Option<String>,
    },
}

impl Default for Body {
    fn default() -> Self {
        Body::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartPart {
    pub name: String,
    pub value: MultipartValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MultipartValue {
    Text {
        content: String,
    },
    File {
        #[serde(with = "base64_string")]
        data: Vec<u8>,
        filename: String,
        mime_type: Option<String>,
    },
}

mod serde_urlencoded_map {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(data: &HashMap<String, String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = serde_urlencoded::to_string(data).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        serde_urlencoded::from_str(&s).map_err(serde::de::Error::custom)
    }
}

mod base64_string {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = STANDARD.encode(data);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

/// Key-value pair for headers, params, etc.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// API Request definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub query_params: Vec<KeyValue>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub body: Body,
    #[serde(default)]
    pub pre_request_script: Option<String>,
    #[serde(default)]
    pub test_script: Option<String>,
    #[serde(default)]
    pub settings: RequestSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSettings {
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub follow_redirects: bool,
    #[serde(default)]
    pub verify_ssl: bool,
    #[serde(default = "default_encoding")]
    pub response_encoding: String,
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout(),
            follow_redirects: true,
            verify_ssl: true,
            response_encoding: default_encoding(),
        }
    }
}

fn default_timeout() -> u64 {
    30000 // 30 seconds
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

impl ApiRequest {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            method: HttpMethod::Get,
            url: url.into(),
            headers: Vec::new(),
            query_params: Vec::new(),
            auth: Auth::None,
            body: Body::None,
            pre_request_script: None,
            test_script: None,
            settings: RequestSettings::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(KeyValue {
            key: key.into(),
            value: value.into(),
            enabled: true,
            description: None,
        });
        self
    }

    pub fn with_body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }
}

/// API Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub status_text: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    pub headers: HashMap<String, String>,
    #[serde(with = "base64_string")]
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub size_bytes: usize,
    pub time_ms: u64,
}

/// Collection of API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub requests: Vec<ApiRequest>,
    pub folders: Vec<CollectionFolder>,
    pub variables: Vec<EnvironmentVariable>,
    pub auth: Option<Auth>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFolder {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub requests: Vec<ApiRequest>,
    #[serde(default)]
    pub folders: Vec<CollectionFolder>,
}

/// Environment variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<EnvironmentVariable>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub request: ApiRequest,
    pub response: ApiResponse,
    pub environment_id: Option<String>,
    pub collection_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub duration_ms: u64,
}

/// WebSocket message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    pub direction: MessageDirection,
    pub content: String,
    #[serde(rename = "type")]
    pub message_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Sent,
    Received,
}

/// gRPC method definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcMethod {
    pub service: String,
    pub method: String,
    pub input_type: String,
    pub output_type: String,
    #[serde(default)]
    pub streaming: bool,
}

/// Import/Export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    PostmanCollection,
    PostmanEnvironment,
    OpenApi,
    Curl,
    Hurl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    PostmanCollection,
    PostmanEnvironment,
    OpenApi,
    Curl,
}

/// API Error types
#[derive(Debug, thiserror::Error, Serialize)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid body: {0}")]
    InvalidBody(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Import error: {0}")]
    Import(String),

    #[error("Export error: {0}")]
    Export(String),
}

pub type ApiResult<T> = Result<T, ApiError>;
