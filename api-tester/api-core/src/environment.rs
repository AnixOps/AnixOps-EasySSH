use crate::types::*;
use std::collections::HashMap;
use regex::Regex;

pub struct EnvironmentManager {
    environments: HashMap<String, Environment>,
    active_environment_id: Option<String>,
}

impl EnvironmentManager {
    pub fn new() -> Self {
        Self {
            environments: HashMap::new(),
            active_environment_id: None,
        }
    }

    pub fn add_environment(&mut self, env: Environment) {
        if env.is_default {
            // Unset default on other environments
            for e in self.environments.values_mut() {
                e.is_default = false;
            }
        }
        self.environments.insert(env.id.clone(), env);
    }

    pub fn get_environment(&self, id: &str) -> Option<&Environment> {
        self.environments.get(id)
    }

    pub fn get_environment_mut(&mut self, id: &str) -> Option<&mut Environment> {
        self.environments.get_mut(id)
    }

    pub fn remove_environment(&mut self, id: &str) -> Option<Environment> {
        self.environments.remove(id)
    }

    pub fn set_active(&mut self, id: Option<String>) {
        self.active_environment_id = id;
    }

    pub fn get_active(&self) -> Option<&Environment> {
        self.active_environment_id
            .as_ref()
            .and_then(|id| self.environments.get(id))
            .or_else(|| self.get_default())
    }

    pub fn get_default(&self) -> Option<&Environment> {
        self.environments.values().find(|e| e.is_default)
    }

    pub fn list_environments(&self) -> Vec<&Environment> {
        let mut envs: Vec<_> = self.environments.values().collect();
        envs.sort_by(|a, b| {
            // Default first, then by name
            if a.is_default != b.is_default {
                b.is_default.cmp(&a.is_default)
            } else {
                a.name.cmp(&b.name)
            }
        });
        envs
    }

    /// Replace variables in a string using {{variable}} syntax
    pub fn replace_variables(&self, input: &str) -> String {
        let pattern = Regex::new(r"\{\{(\w+)\}\}").unwrap();
        let env = self.get_active();

        if let Some(env) = env {
            let var_map: HashMap<_, _> = env.variables
                .iter()
                .filter(|v| v.enabled)
                .map(|v| (v.key.clone(), v.value.clone()))
                .collect();

            pattern.replace_all(input, |caps: &regex::Captures| {
                let var_name = &caps[1];
                var_map.get(var_name)
                    .cloned()
                    .unwrap_or_else(|| caps[0].to_string())
            }).to_string()
        } else {
            input.to_string()
        }
    }

    /// Apply environment variables to a request
    pub fn apply_to_request(&self, request: &mut ApiRequest) {
        // Replace URL variables
        request.url = self.replace_variables(&request.url);

        // Replace header values
        for header in &mut request.headers {
            header.value = self.replace_variables(&header.value);
        }

        // Replace query param values
        for param in &mut request.query_params {
            param.value = self.replace_variables(&param.value);
        }

        // Replace body content
        match &mut request.body {
            Body::Text { content } => {
                *content = self.replace_variables(content);
            }
            Body::Json { content } => {
                *content = self.replace_variables(content);
            }
            Body::Xml { content } => {
                *content = self.replace_variables(content);
            }
            _ => {}
        }

        // Replace auth values
        match &mut request.auth {
            Auth::Basic { username, password } => {
                *username = self.replace_variables(username);
                *password = self.replace_variables(password);
            }
            Auth::Bearer { token } => {
                *token = self.replace_variables(token);
            }
            Auth::ApiKey { key, value, .. } => {
                *key = self.replace_variables(key);
                *value = self.replace_variables(value);
            }
            Auth::Oauth2 { access_token, refresh_token } => {
                *access_token = self.replace_variables(access_token);
                if let Some(rt) = refresh_token {
                    *rt = self.replace_variables(rt);
                }
            }
            _ => {}
        }
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_variables() {
        let mut manager = EnvironmentManager::new();

        let env = Environment {
            id: "env1".to_string(),
            name: "Development".to_string(),
            variables: vec![
                EnvironmentVariable {
                    key: "base_url".to_string(),
                    value: "https://api.example.com".to_string(),
                    enabled: true,
                    description: None,
                },
                EnvironmentVariable {
                    key: "api_key".to_string(),
                    value: "secret123".to_string(),
                    enabled: true,
                    description: None,
                },
            ],
            is_default: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        manager.add_environment(env);
        manager.set_active(Some("env1".to_string()));

        let result = manager.replace_variables("{{base_url}}/users?api_key={{api_key}}");
        assert_eq!(result, "https://api.example.com/users?api_key=secret123");
    }

    #[test]
    fn test_apply_to_request() {
        let mut manager = EnvironmentManager::new();

        let env = Environment {
            id: "env1".to_string(),
            name: "Development".to_string(),
            variables: vec![
                EnvironmentVariable {
                    key: "host".to_string(),
                    value: "api.dev.com".to_string(),
                    enabled: true,
                    description: None,
                },
            ],
            is_default: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        manager.add_environment(env);

        let mut request = ApiRequest::new("Test", "https://{{host}}/api");
        manager.apply_to_request(&mut request);

        assert_eq!(request.url, "https://api.dev.com/api");
    }
}
