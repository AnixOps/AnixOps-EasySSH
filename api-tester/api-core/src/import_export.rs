use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Importer;
pub struct Exporter;

impl Importer {
    pub fn new() -> Self {
        Self
    }

    /// Import Postman collection v2.1
    pub fn import_postman_collection(&self, data: &str) -> ApiResult<Collection> {
        let postman: PostmanCollectionV2 = serde_json::from_str(data)
            .map_err(|e| ApiError::Import(format!("Invalid Postman collection: {}", e)))?;

        let mut collection = Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: postman.info.name,
            description: postman.info.description,
            requests: Vec::new(),
            folders: Vec::new(),
            variables: Vec::new(),
            auth: postman.auth.map(|a| self.convert_auth(&a)),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Process items (requests and folders)
        for item in &postman.item {
            self.process_postman_item(item, &mut collection, None)?;
        }

        Ok(collection)
    }

    fn process_postman_item(
        &self,
        item: &PostmanItem,
        collection: &mut Collection,
        parent_id: Option<&str>,
    ) -> ApiResult<()> {
        if item.request.is_some() {
            // This is a request
            let request = self.convert_request(item)?;
            if let Some(folder_id) = parent_id {
                // Add to folder
                for folder in &mut collection.folders {
                    if folder.id == folder_id {
                        folder.requests.push(request);
                        return Ok(());
                    }
                }
            } else {
                collection.requests.push(request);
            }
        } else if !item.item.is_empty() {
            // This is a folder
            let folder = CollectionFolder {
                id: uuid::Uuid::new_v4().to_string(),
                name: item.name.clone(),
                description: item.description.clone(),
                requests: Vec::new(),
                folders: Vec::new(),
            };

            let folder_id = folder.id.clone();

            if let Some(parent) = parent_id {
                // Add to parent folder (nested)
                for f in &mut collection.folders {
                    if f.id == parent {
                        f.folders.push(folder);
                        break;
                    }
                }
            } else {
                collection.folders.push(folder);
            }

            // Process children
            for child in &item.item {
                self.process_postman_item(child, collection, Some(&folder_id))?;
            }
        }

        Ok(())
    }

    fn convert_request(&self, item: &PostmanItem) -> ApiResult<ApiRequest> {
        let request = item.request.as_ref()
            .ok_or_else(|| ApiError::Import("Request data missing".to_string()))?;

        let method = match request.method.as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            _ => HttpMethod::Get,
        };

        let url = self.convert_url(&request.url);

        let headers = request.header.as_ref()
            .map(|h| h.iter().map(|kv| KeyValue {
                key: kv.key.clone(),
                value: kv.value.clone(),
                enabled: !kv.disabled.unwrap_or(false),
                description: kv.description.clone(),
            }).collect())
            .unwrap_or_default();

        let query_params = request.url.query.as_ref()
            .map(|q| q.iter().map(|kv| KeyValue {
                key: kv.key.clone(),
                value: kv.value.clone(),
                enabled: !kv.disabled.unwrap_or(false),
                description: kv.description.clone(),
            }).collect())
            .unwrap_or_default();

        let auth = request.auth.as_ref().map(|a| self.convert_auth(a));
        let body = request.body.as_ref().map(|b| self.convert_body(b)).unwrap_or_default();

        Ok(ApiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            name: item.name.clone(),
            method,
            url,
            headers,
            query_params,
            auth: auth.unwrap_or_default(),
            body,
            pre_request_script: None,
            test_script: item.event.as_ref()
                .and_then(|e| e.iter().find(|ev| ev.listen == "test"))
                .and_then(|ev| ev.script.exec.as_ref())
                .map(|lines| lines.join("\n")),
            settings: RequestSettings::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    fn convert_url(&self, url: &PostmanUrl) -> String {
        if let Some(raw) = &url.raw {
            return raw.clone();
        }

        // Build URL from parts
        let protocol = url.protocol.as_deref().unwrap_or("https");
        let host = url.host.as_ref()
            .map(|h| h.iter().map(|p| &p.value).cloned().collect::<Vec<_>>().join("."))
            .unwrap_or_default();
        let port = url.port.as_deref().unwrap_or("");
        let path = url.path.as_ref()
            .map(|p| p.iter().map(|part| &part.value).cloned().collect::<Vec<_>>().join("/"))
            .unwrap_or_default();

        if port.is_empty() {
            format!("{}://{}/{}", protocol, host, path)
        } else {
            format!("{}://{}:{}/{}", protocol, host, port, path)
        }
    }

    fn convert_auth(&self, auth: &PostmanAuth) -> Auth {
        match auth.auth_type.as_str() {
            "basic" => {
                let username = auth.basic.as_ref()
                    .and_then(|b| b.iter().find(|kv| kv.key == "username"))
                    .map(|kv| kv.value.clone())
                    .unwrap_or_default();
                let password = auth.basic.as_ref()
                    .and_then(|b| b.iter().find(|kv| kv.key == "password"))
                    .map(|kv| kv.value.clone())
                    .unwrap_or_default();
                Auth::Basic { username, password }
            }
            "bearer" => {
                let token = auth.bearer.as_ref()
                    .and_then(|b| b.iter().find(|kv| kv.key == "token"))
                    .map(|kv| kv.value.clone())
                    .unwrap_or_default();
                Auth::Bearer { token }
            }
            _ => Auth::None,
        }
    }

    fn convert_body(&self, body: &PostmanBody) -> Body {
        match body.mode.as_str() {
            "raw" => {
                let content = body.raw.clone().unwrap_or_default();
                match body.options.as_ref()
                    .and_then(|o| o.raw.as_ref())
                    .and_then(|r| r.language.as_ref())
                    .map(|l| l.as_str()) {
                    Some("json") => Body::Json { content },
                    Some("xml") => Body::Xml { content },
                    _ => Body::Text { content },
                }
            }
            "formdata" => {
                let parts: Vec<MultipartPart> = body.formdata.as_ref()
                    .map(|f| f.iter().map(|kv| MultipartPart {
                        name: kv.key.clone(),
                        value: MultipartValue::Text { content: kv.value.clone() },
                    }).collect())
                    .unwrap_or_default();
                Body::Multipart { parts }
            }
            "urlencoded" => {
                let data: HashMap<String, String> = body.urlencoded.as_ref()
                    .map(|u| u.iter()
                        .filter(|kv| !kv.disabled.unwrap_or(false))
                        .map(|kv| (kv.key.clone(), kv.value.clone()))
                        .collect())
                    .unwrap_or_default();
                Body::Form { data }
            }
            _ => Body::None,
        }
    }

    /// Import Postman environment
    pub fn import_postman_environment(&self, data: &str) -> ApiResult<Environment> {
        let postman_env: PostmanEnvironment = serde_json::from_str(data)
            .map_err(|e| ApiError::Import(format!("Invalid Postman environment: {}", e)))?;

        let variables: Vec<EnvironmentVariable> = postman_env.values
            .iter()
            .map(|v| EnvironmentVariable {
                key: v.key.clone(),
                value: v.value.clone(),
                enabled: !v.disabled,
                description: None,
            })
            .collect();

        Ok(Environment {
            id: uuid::Uuid::new_v4().to_string(),
            name: postman_env.name,
            variables,
            is_default: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Import from curl command
    pub fn import_curl(&self, command: &str) -> ApiResult<ApiRequest> {
        // Parse curl command
        // curl -X POST -H "Content-Type: application/json" -d '{"key":"value"}' https://api.example.com

        let mut request = ApiRequest::new("Imported Request", "");

        // Method
        if let Some(start) = command.find("-X ") {
            let rest = &command[start + 3..];
            let end = rest.find(' ').unwrap_or(rest.len());
            let method_str = &rest[..end];
            request.method = match method_str.to_uppercase().as_str() {
                "GET" => HttpMethod::Get,
                "POST" => HttpMethod::Post,
                "PUT" => HttpMethod::Put,
                "DELETE" => HttpMethod::Delete,
                "PATCH" => HttpMethod::Patch,
                _ => HttpMethod::Get,
            };
        } else if command.contains("-d ") || command.contains("--data ") {
            request.method = HttpMethod::Post;
        }

        // Headers
        let header_regex = regex::Regex::new(r#"-H\s+['"]([^'"]+)['"]"#).unwrap();
        for cap in header_regex.captures_iter(command) {
            let header_str = &cap[1];
            let parts: Vec<&str> = header_str.splitn(2, ':').collect();
            if parts.len() == 2 {
                request.headers.push(KeyValue {
                    key: parts[0].trim().to_string(),
                    value: parts[1].trim().to_string(),
                    enabled: true,
                    description: None,
                });
            }
        }

        // Body
        let data_regex = regex::Regex::new(r#"-d\s+['"]([^'"]+)['"]"#).unwrap();
        if let Some(cap) = data_regex.captures(command) {
            let data = &cap[1];
            // Try to detect if JSON
            if data.trim().starts_with('{') || data.trim().starts_with('[') {
                request.body = Body::Json { content: data.to_string() };
            } else {
                request.body = Body::Form {
                    data: serde_urlencoded::from_str(data).unwrap_or_default(),
                };
            }
        }

        // URL - extract last argument
        let parts: Vec<&str> = command.split_whitespace().collect();
        if let Some(url) = parts.last() {
            if url.starts_with("http") {
                request.url = url.to_string();
            }
        }

        Ok(request)
    }

    /// Import OpenAPI specification
    pub fn import_openapi(&self, data: &str) -> ApiResult<Vec<Collection>> {
        // This is a simplified implementation
        // Full OpenAPI import would parse paths, methods, parameters, schemas
        Err(ApiError::Import("OpenAPI import not fully implemented".to_string()))
    }
}

impl Exporter {
    pub fn new() -> Self {
        Self
    }

    /// Export to Postman collection v2.1
    pub fn export_postman_collection(&self, collection: &Collection) -> ApiResult<String> {
        let postman = PostmanCollectionV2 {
            info: PostmanInfo {
                name: collection.name.clone(),
                description: collection.description.clone(),
                schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json".to_string(),
            },
            item: self.convert_to_items(collection),
            auth: collection.auth.as_ref().map(|a| self.convert_auth(a)),
            variable: if collection.variables.is_empty() {
                None
            } else {
                Some(collection.variables.iter().map(|v| PostmanVariable {
                    key: v.key.clone(),
                    value: v.value.clone(),
                    disabled: Some(!v.enabled),
                }).collect())
            },
        };

        serde_json::to_string_pretty(&postman)
            .map_err(|e| ApiError::Export(e.to_string()))
    }

    fn convert_to_items(&self, collection: &Collection) -> Vec<PostmanItem> {
        let mut items = Vec::new();

        // Add root requests
        for request in &collection.requests {
            items.push(self.convert_request_to_item(request));
        }

        // Add folders
        for folder in &collection.folders {
            items.push(self.convert_folder_to_item(folder));
        }

        items
    }

    fn convert_request_to_item(&self, request: &ApiRequest) -> PostmanItem {
        PostmanItem {
            name: request.name.clone(),
            description: None,
            item: Vec::new(),
            request: Some(PostmanRequestDetail {
                method: request.method.to_string(),
                header: Some(request.headers.iter().map(|h| PostmanHeader {
                    key: h.key.clone(),
                    value: h.value.clone(),
                    disabled: Some(!h.enabled),
                    description: h.description.clone(),
                }).collect()),
                body: self.convert_body_to_postman(&request.body),
                url: PostmanUrl {
                    raw: Some(request.url.clone()),
                    protocol: None,
                    host: None,
                    port: None,
                    path: None,
                    query: if request.query_params.is_empty() {
                        None
                    } else {
                        Some(request.query_params.iter().map(|p| PostmanQueryParam {
                            key: p.key.clone(),
                            value: p.value.clone(),
                            disabled: Some(!p.enabled),
                            description: p.description.clone(),
                        }).collect())
                    },
                },
                auth: if matches!(request.auth, Auth::None) {
                    None
                } else {
                    Some(self.convert_auth_to_postman(&request.auth))
                },
            }),
            event: request.test_script.as_ref().map(|script| vec![PostmanEvent {
                listen: "test".to_string(),
                script: PostmanScript {
                    exec: Some(script.lines().map(|s| s.to_string()).collect()),
                },
            }]),
        }
    }

    fn convert_folder_to_item(&self, folder: &CollectionFolder) -> PostmanItem {
        let mut item = PostmanItem {
            name: folder.name.clone(),
            description: folder.description.clone(),
            item: Vec::new(),
            request: None,
            event: None,
        };

        // Add requests in folder
        for request in &folder.requests {
            item.item.push(self.convert_request_to_item(request));
        }

        // Add sub-folders
        for sub_folder in &folder.folders {
            item.item.push(self.convert_folder_to_item(sub_folder));
        }

        item
    }

    fn convert_body_to_postman(&self, body: &Body) -> Option<PostmanBody> {
        match body {
            Body::None => None,
            Body::Text { content } => Some(PostmanBody {
                mode: "raw".to_string(),
                raw: Some(content.clone()),
                formdata: None,
                urlencoded: None,
                options: Some(PostmanBodyOptions {
                    raw: Some(PostmanRawOptions { language: Some("text".to_string()) }),
                }),
            }),
            Body::Json { content } => Some(PostmanBody {
                mode: "raw".to_string(),
                raw: Some(content.clone()),
                formdata: None,
                urlencoded: None,
                options: Some(PostmanBodyOptions {
                    raw: Some(PostmanRawOptions { language: Some("json".to_string()) }),
                }),
            }),
            Body::Xml { content } => Some(PostmanBody {
                mode: "raw".to_string(),
                raw: Some(content.clone()),
                formdata: None,
                urlencoded: None,
                options: Some(PostmanBodyOptions {
                    raw: Some(PostmanRawOptions { language: Some("xml".to_string()) }),
                }),
            }),
            Body::Form { data } => Some(PostmanBody {
                mode: "urlencoded".to_string(),
                raw: None,
                formdata: None,
                urlencoded: Some(data.iter().map(|(k, v)| PostmanQueryParam {
                    key: k.clone(),
                    value: v.clone(),
                    disabled: Some(false),
                    description: None,
                }).collect()),
                options: None,
            }),
            Body::Multipart { parts } => Some(PostmanBody {
                mode: "formdata".to_string(),
                raw: None,
                formdata: Some(parts.iter().map(|p| PostmanQueryParam {
                    key: p.name.clone(),
                    value: match &p.value {
                        MultipartValue::Text { content } => content.clone(),
                        MultipartValue::File { filename, .. } => filename.clone(),
                    },
                    disabled: Some(false),
                    description: None,
                }).collect()),
                urlencoded: None,
                options: None,
            }),
            Body::Binary { .. } => None,
        }
    }

    fn convert_auth(&self, auth: &Auth) -> PostmanAuth {
        PostmanAuth {
            auth_type: match auth {
                Auth::None => "noauth",
                Auth::Basic { .. } => "basic",
                Auth::Bearer { .. } => "bearer",
                Auth::ApiKey { .. } => "apikey",
                Auth::Oauth2 { .. } => "oauth2",
                Auth::Digest { .. } => "digest",
            }.to_string(),
            basic: match auth {
                Auth::Basic { username, password } => Some(vec![
                    PostmanAuthParam { key: "username".to_string(), value: username.clone() },
                    PostmanAuthParam { key: "password".to_string(), value: password.clone() },
                ]),
                _ => None,
            },
            bearer: match auth {
                Auth::Bearer { token } => Some(vec![
                    PostmanAuthParam { key: "token".to_string(), value: token.clone() },
                ]),
                _ => None,
            },
        }
    }

    fn convert_auth_to_postman(&self, auth: &Auth) -> PostmanAuth {
        self.convert_auth(auth)
    }

    /// Export to Postman environment
    pub fn export_postman_environment(&self, env: &Environment) -> ApiResult<String> {
        let postman_env = PostmanEnvironment {
            name: env.name.clone(),
            values: env.variables.iter().map(|v| PostmanEnvValue {
                key: v.key.clone(),
                value: v.value.clone(),
                disabled: !v.enabled,
            }).collect(),
        };

        serde_json::to_string_pretty(&postman_env)
            .map_err(|e| ApiError::Export(e.to_string()))
    }

    /// Export to curl command
    pub fn export_curl(&self, request: &ApiRequest) -> ApiResult<String> {
        let mut cmd = format!("curl -X {} ", request.method.to_string());

        // Headers
        for header in &request.headers {
            if header.enabled {
                cmd.push_str(&format!("-H \"{}: {}\" ", header.key, header.value));
            }
        }

        // Body
        match &request.body {
            Body::Text { content } => {
                cmd.push_str(&format!("-d '{}' ", content.replace("'", "'\"'\"'")));
            }
            Body::Json { content } => {
                cmd.push_str(&format!("-H \"Content-Type: application/json\" -d '{}' ", content.replace("'", "'\"'\"'")));
            }
            Body::Form { data } => {
                let form_str = serde_urlencoded::to_string(data).unwrap_or_default();
                cmd.push_str(&format!("-d '{}' ", form_str));
            }
            _ => {}
        }

        // Auth
        match &request.auth {
            Auth::Basic { username, password } => {
                cmd.push_str(&format!("-u '{}:{}' ", username, password));
            }
            Auth::Bearer { token } => {
                cmd.push_str(&format!("-H \"Authorization: Bearer {}\" ", token));
            }
            _ => {}
        }

        // URL
        cmd.push_str(&format!("\"{}\"", request.url));

        Ok(cmd)
    }
}

// Postman v2.1 format structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanCollectionV2 {
    info: PostmanInfo,
    item: Vec<PostmanItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<PostmanAuth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variable: Option<Vec<PostmanVariable>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanInfo {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    schema: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanItem {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default)]
    item: Vec<PostmanItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request: Option<PostmanRequestDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    event: Option<Vec<PostmanEvent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanRequestDetail {
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    header: Option<Vec<PostmanHeader>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<PostmanBody>,
    url: PostmanUrl,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<PostmanAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanBody {
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formdata: Option<Vec<PostmanQueryParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    urlencoded: Option<Vec<PostmanQueryParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<PostmanBodyOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanBodyOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<PostmanRawOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanRawOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanUrl {
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<Vec<PostmanUrlPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<Vec<PostmanUrlPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<Vec<PostmanQueryParam>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanUrlPart {
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanQueryParam {
    key: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanAuth {
    #[serde(rename = "type")]
    auth_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    basic: Option<Vec<PostmanAuthParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bearer: Option<Vec<PostmanAuthParam>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanAuthParam {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanEvent {
    listen: String,
    script: PostmanScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanScript {
    #[serde(skip_serializing_if = "Option::is_none")]
    exec: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanVariable {
    key: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanEnvironment {
    name: String,
    values: Vec<PostmanEnvValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostmanEnvValue {
    key: String,
    value: String,
    disabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_postman_collection() {
        let json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get Users",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": {
                            "raw": "https://api.example.com/users"
                        }
                    }
                }
            ]
        }"#;

        let importer = Importer::new();
        let collection = importer.import_postman_collection(json).unwrap();
        assert_eq!(collection.name, "Test Collection");
        assert_eq!(collection.requests.len(), 1);
        assert_eq!(collection.requests[0].name, "Get Users");
    }

    #[test]
    fn test_import_curl() {
        let cmd = r#"curl -X POST -H "Content-Type: application/json" -d '{"name":"test"}' https://api.example.com/users"#;

        let importer = Importer::new();
        let request = importer.import_curl(cmd).unwrap();
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "https://api.example.com/users");
    }
}
