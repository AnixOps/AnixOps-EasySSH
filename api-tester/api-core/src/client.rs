use crate::types::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> ApiResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| ApiError::Network(e.to_string()))?;

        Ok(Self { client })
    }

    pub fn with_timeout(timeout_ms: u64) -> ApiResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| ApiError::Network(e.to_string()))?;

        Ok(Self { client })
    }

    pub async fn execute(&self, request: &ApiRequest) -> ApiResult<ApiResponse> {
        let start = Instant::now();

        // Parse and build URL with query params
        let url = self.build_url(&request.url, &request.query_params)?;

        // Build request method
        let method = self.parse_method(&request.method);

        // Build request builder
        let mut req_builder = self.client.request(method, url);

        // Add headers
        let headers = self.build_headers(&request.headers, &request.auth)?;
        req_builder = req_builder.headers(headers);

        // Add body if present
        req_builder = self.add_body(req_builder, &request.body).await?;

        // Execute request
        let response = req_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ApiError::Timeout
            } else {
                ApiError::Network(e.to_string())
            }
        })?;

        let time_ms = start.elapsed().as_millis() as u64;

        // Extract response info
        let status = response.status().as_u16();
        let status_text = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();

        // Extract headers
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(val) = value.to_str() {
                headers.insert(key.to_string(), val.to_string());
            }
        }

        // Get content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Read body
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let size_bytes = body_bytes.len();
        let body = body_bytes.to_vec();

        Ok(ApiResponse {
            status,
            status_text,
            timestamp: chrono::Utc::now(),
            headers,
            body,
            content_type,
            size_bytes,
            time_ms,
        })
    }

    fn build_url(&self, base_url: &str, params: &[KeyValue]) -> ApiResult<Url> {
        let mut url = Url::parse(base_url).map_err(|e| ApiError::InvalidUrl(e.to_string()))?;

        // Add query parameters
        let mut query_pairs = url.query_pairs_mut();
        for param in params {
            if param.enabled && !param.key.is_empty() {
                query_pairs.append_pair(&param.key, &param.value);
            }
        }
        drop(query_pairs);

        Ok(url)
    }

    fn parse_method(&self, method: &HttpMethod) -> Method {
        match method {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Delete => Method::DELETE,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Head => Method::HEAD,
            HttpMethod::Options => Method::OPTIONS,
            HttpMethod::Connect => Method::CONNECT,
            HttpMethod::Trace => Method::TRACE,
        }
    }

    fn build_headers(&self, headers: &[KeyValue], auth: &Auth) -> ApiResult<HeaderMap> {
        let mut header_map = HeaderMap::new();

        // Add custom headers
        for header in headers {
            if header.enabled && !header.key.is_empty() {
                let name = HeaderName::from_bytes(header.key.as_bytes())
                    .map_err(|e| ApiError::InvalidBody(format!("Invalid header name: {}", e)))?;
                let value = HeaderValue::from_str(&header.value)
                    .map_err(|e| ApiError::InvalidBody(format!("Invalid header value: {}", e)))?;
                header_map.insert(name, value);
            }
        }

        // Add auth headers
        match auth {
            Auth::None => {}
            Auth::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                let encoded = STANDARD.encode(credentials);
                let auth_value = format!("Basic {}", encoded);
                header_map.insert(
                    "Authorization",
                    HeaderValue::from_str(&auth_value)
                        .map_err(|e| ApiError::Auth(e.to_string()))?,
                );
            }
            Auth::Bearer { token } => {
                let auth_value = format!("Bearer {}", token);
                header_map.insert(
                    "Authorization",
                    HeaderValue::from_str(&auth_value)
                        .map_err(|e| ApiError::Auth(e.to_string()))?,
                );
            }
            Auth::ApiKey { key, value, in_ } if in_ == "header" => {
                header_map.insert(
                    HeaderName::from_bytes(key.as_bytes())
                        .map_err(|e| ApiError::Auth(e.to_string()))?,
                    HeaderValue::from_str(value).map_err(|e| ApiError::Auth(e.to_string()))?,
                );
            }
            _ => {}
        }

        Ok(header_map)
    }

    async fn add_body(
        &self,
        builder: reqwest::RequestBuilder,
        body: &Body,
    ) -> ApiResult<reqwest::RequestBuilder> {
        match body {
            Body::None => Ok(builder),
            Body::Text { content } => Ok(builder.body(content.clone())),
            Body::Json { content } => {
                // Validate JSON
                serde_json::from_str::<serde_json::Value>(content)
                    .map_err(|e| ApiError::InvalidBody(format!("Invalid JSON: {}", e)))?;
                Ok(builder
                    .header("Content-Type", "application/json")
                    .body(content.clone()))
            }
            Body::Xml { content } => Ok(builder
                .header("Content-Type", "application/xml")
                .body(content.clone())),
            Body::Form { data } => {
                let form_data: HashMap<String, String> = data.clone();
                Ok(builder
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .form(&form_data))
            }
            Body::Multipart { parts } => {
                let mut form = reqwest::multipart::Form::new();
                for part in parts {
                    match &part.value {
                        MultipartValue::Text { content } => {
                            form = form.text(part.name.clone(), content.clone());
                        }
                        MultipartValue::File {
                            data,
                            filename,
                            mime_type,
                        } => {
                            let mut file_part = reqwest::multipart::Part::bytes(data.clone())
                                .file_name(filename.clone());
                            if let Some(mime) = mime_type {
                                file_part = file_part
                                    .mime_str(mime)
                                    .map_err(|e| ApiError::InvalidBody(e.to_string()))?;
                            }
                            form = form.part(part.name.clone(), file_part);
                        }
                    }
                }
                Ok(builder.multipart(form))
            }
            Body::Binary {
                data,
                filename,
                mime_type,
            } => {
                let mut part = reqwest::multipart::Part::bytes(data.clone());
                if let Some(name) = filename {
                    part = part.file_name(name.clone());
                }
                if let Some(mime) = mime_type {
                    part = part
                        .mime_str(mime)
                        .map_err(|e| ApiError::InvalidBody(e.to_string()))?;
                }
                let form = reqwest::multipart::Form::new().part("file", part);
                Ok(builder.multipart(form))
            }
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create HTTP client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_url_with_params() {
        let client = HttpClient::new().unwrap();
        let params = vec![
            KeyValue {
                key: "foo".to_string(),
                value: "bar".to_string(),
                enabled: true,
                description: None,
            },
            KeyValue {
                key: "baz".to_string(),
                value: "qux".to_string(),
                enabled: true,
                description: None,
            },
        ];

        let url = client
            .build_url("https://example.com/api", &params)
            .unwrap();
        assert_eq!(url.as_str(), "https://example.com/api?foo=bar&baz=qux");
    }

    #[test]
    fn test_parse_method() {
        let client = HttpClient::new().unwrap();
        assert_eq!(client.parse_method(&HttpMethod::Get).as_str(), "GET");
        assert_eq!(client.parse_method(&HttpMethod::Post).as_str(), "POST");
        assert_eq!(client.parse_method(&HttpMethod::Put).as_str(), "PUT");
    }
}
