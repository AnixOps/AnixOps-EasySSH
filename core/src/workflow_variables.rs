use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Workflow script variable types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Server,  // Special type for server references
}

/// Script variable with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptVariable {
    pub name: String,
    pub value: serde_json::Value,
    pub var_type: VariableType,
    pub description: Option<String>,
    pub is_required: bool,
    pub default_value: Option<serde_json::Value>,
}

impl ScriptVariable {
    pub fn new(name: &str, value: serde_json::Value, var_type: VariableType) -> Self {
        Self {
            name: name.to_string(),
            value,
            var_type,
            description: None,
            is_required: false,
            default_value: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn required(mut self) -> Self {
        self.is_required = true;
        self
    }

    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default_value = Some(default);
        self
    }
}

/// Built-in variable templates
pub struct VariableTemplates;

impl VariableTemplates {
    /// Get all available variable templates for server context
    pub fn server_variables() -> Vec<ScriptVariable> {
        vec![
            ScriptVariable::new(
                "server.id",
                serde_json::Value::String("{{server.id}}".to_string()),
                VariableType::String,
            ).with_description("Server unique identifier"),
            ScriptVariable::new(
                "server.name",
                serde_json::Value::String("{{server.name}}".to_string()),
                VariableType::String,
            ).with_description("Server display name"),
            ScriptVariable::new(
                "server.host",
                serde_json::Value::String("{{server.host}}".to_string()),
                VariableType::String,
            ).with_description("Server hostname or IP"),
            ScriptVariable::new(
                "server.port",
                serde_json::Value::Number(22.into()),
                VariableType::Number,
            ).with_description("SSH port"),
            ScriptVariable::new(
                "server.username",
                serde_json::Value::String("{{server.username}}".to_string()),
                VariableType::String,
            ).with_description("SSH username"),
            ScriptVariable::new(
                "server.password",
                serde_json::Value::String("{{server.password}}".to_string()),
                VariableType::String,
            ).with_description("SSH password (encrypted)"),
            ScriptVariable::new(
                "server.key_path",
                serde_json::Value::String("{{server.key_path}}".to_string()),
                VariableType::String,
            ).with_description("SSH private key path"),
            ScriptVariable::new(
                "server.group",
                serde_json::Value::String("{{server.group}}".to_string()),
                VariableType::String,
            ).with_description("Server group name"),
            ScriptVariable::new(
                "server.tags",
                serde_json::Value::Array(vec![]),
                VariableType::Array,
            ).with_description("Server tags"),
        ]
    }

    /// Get system variables
    pub fn system_variables() -> Vec<ScriptVariable> {
        vec![
            ScriptVariable::new(
                "system.timestamp",
                serde_json::Value::String("{{system.timestamp}}".to_string()),
                VariableType::String,
            ).with_description("Current timestamp ISO8601"),
            ScriptVariable::new(
                "system.date",
                serde_json::Value::String("{{system.date}}".to_string()),
                VariableType::String,
            ).with_description("Current date YYYY-MM-DD"),
            ScriptVariable::new(
                "system.time",
                serde_json::Value::String("{{system.time}}".to_string()),
                VariableType::String,
            ).with_description("Current time HH:MM:SS"),
            ScriptVariable::new(
                "system.random",
                serde_json::Value::String("{{system.random}}".to_string()),
                VariableType::String,
            ).with_description("Random string"),
            ScriptVariable::new(
                "system.uuid",
                serde_json::Value::String("{{system.uuid}}".to_string()),
                VariableType::String,
            ).with_description("Unique identifier"),
        ]
    }

    /// Get execution context variables
    pub fn execution_variables() -> Vec<ScriptVariable> {
        vec![
            ScriptVariable::new(
                "execution.id",
                serde_json::Value::String("{{execution.id}}".to_string()),
                VariableType::String,
            ).with_description("Execution run ID"),
            ScriptVariable::new(
                "execution.start_time",
                serde_json::Value::String("{{execution.start_time}}".to_string()),
                VariableType::String,
            ).with_description("Execution start time"),
            ScriptVariable::new(
                "execution.parallel_index",
                serde_json::Value::Number(0.into()),
                VariableType::Number,
            ).with_description("Index in parallel execution"),
            ScriptVariable::new(
                "execution.total_servers",
                serde_json::Value::Number(1.into()),
                VariableType::Number,
            ).with_description("Total servers in batch"),
            ScriptVariable::new(
                "execution.previous_result",
                serde_json::Value::String("{{execution.previous_result}}".to_string()),
                VariableType::String,
            ).with_description("Previous step output"),
            ScriptVariable::new(
                "execution.exit_code",
                serde_json::Value::Number(0.into()),
                VariableType::Number,
            ).with_description("Last command exit code"),
        ]
    }
}

/// Variable resolver that substitutes template variables with actual values
pub struct VariableResolver {
    variables: HashMap<String, serde_json::Value>,
    server_context: Option<ServerContext>,
}

#[derive(Clone, Debug)]
pub struct ServerContext {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub group: Option<String>,
    pub tags: Vec<String>,
}

impl VariableResolver {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            server_context: None,
        }
    }

    pub fn with_server(mut self, server: ServerContext) -> Self {
        // Add server variables
        self.variables.insert("server.id".to_string(), server.id.clone().into());
        self.variables.insert("server.name".to_string(), server.name.clone().into());
        self.variables.insert("server.host".to_string(), server.host.clone().into());
        self.variables.insert("server.port".to_string(), (server.port as i64).into());
        self.variables.insert("server.username".to_string(), server.username.clone().into());

        if let Some(ref pwd) = server.password {
            self.variables.insert("server.password".to_string(), pwd.clone().into());
        }
        if let Some(ref key) = server.key_path {
            self.variables.insert("server.key_path".to_string(), key.clone().into());
        }
        if let Some(ref group) = server.group {
            self.variables.insert("server.group".to_string(), group.clone().into());
        }

        self.variables.insert("server.tags".to_string(),
            server.tags.clone().into_iter().map(|t: String| t.into()).collect::<Vec<serde_json::Value>>().into());

        self.server_context = Some(server);
        self
    }

    pub fn with_system_variables(mut self) -> Self {
        let now = Utc::now();

        self.variables.insert("system.timestamp".to_string(),
            now.to_rfc3339().into());
        self.variables.insert("system.date".to_string(),
            now.format("%Y-%m-%d").to_string().into());
        self.variables.insert("system.time".to_string(),
            now.format("%H:%M:%S").to_string().into());
        self.variables.insert("system.random".to_string(),
            Uuid::new_v4().to_string().into());
        self.variables.insert("system.uuid".to_string(),
            Uuid::new_v4().to_string().into());

        self
    }

    pub fn with_execution_context(mut self, context: ExecutionContext) -> Self {
        self.variables.insert("execution.id".to_string(), context.execution_id.into());
        self.variables.insert("execution.start_time".to_string(),
            context.start_time.to_rfc3339().into());
        self.variables.insert("execution.parallel_index".to_string(),
            (context.parallel_index as i64).into());
        self.variables.insert("execution.total_servers".to_string(),
            (context.total_servers as i64).into());

        if let Some(prev) = context.previous_result {
            self.variables.insert("execution.previous_result".to_string(), prev.into());
        }
        self.variables.insert("execution.exit_code".to_string(),
            (context.exit_code as i64).into());

        self
    }

    pub fn add_variable(&mut self, name: &str, value: impl Into<serde_json::Value>) {
        self.variables.insert(name.to_string(), value.into());
    }

    /// Resolve all variables in a string template
    pub fn resolve(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Replace {{variable.name}} patterns
        for (key, value) in &self.variables {
            let pattern = format!("{{{{{}}}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Array(arr) => {
                    arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")
                }
                serde_json::Value::Object(_) => value.to_string(),
                serde_json::Value::Null => String::new(),
            };
            result = result.replace(&pattern, &replacement);
        }

        result
    }

    /// Resolve variables in a JSON value
    pub fn resolve_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                serde_json::Value::String(self.resolve(s))
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(
                    arr.iter().map(|v| self.resolve_json(v)).collect()
                )
            }
            serde_json::Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    let resolved_key = self.resolve(k);
                    new_obj.insert(resolved_key, self.resolve_json(v));
                }
                serde_json::Value::Object(new_obj)
            }
            _ => value.clone(),
        }
    }

    /// Get a variable value
    pub fn get(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// Get all variables
    pub fn get_all(&self) -> &HashMap<String, serde_json::Value> {
        &self.variables
    }
}

#[derive(Clone, Debug)]
pub struct ExecutionContext {
    pub execution_id: String,
    pub start_time: DateTime<Utc>,
    pub parallel_index: usize,
    pub total_servers: usize,
    pub previous_result: Option<String>,
    pub exit_code: i32,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            execution_id: Uuid::new_v4().to_string(),
            start_time: Utc::now(),
            parallel_index: 0,
            total_servers: 1,
            previous_result: None,
            exit_code: 0,
        }
    }
}

/// Variable validation
pub struct VariableValidator;

impl VariableValidator {
    /// Validate that all required variables are present
    pub fn validate_required(
        variables: &[ScriptVariable],
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<(), Vec<String>> {
        let mut missing = Vec::new();

        for var in variables {
            if var.is_required && !context.contains_key(&var.name) {
                missing.push(var.name.clone());
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Validate variable types
    pub fn validate_types(
        variables: &[ScriptVariable],
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<(), Vec<(String, String)>> {
        let mut errors = Vec::new();

        for var in variables {
            if let Some(value) = context.get(&var.name) {
                let valid = match var.var_type {
                    VariableType::String => value.is_string(),
                    VariableType::Number => value.is_number(),
                    VariableType::Boolean => value.is_boolean(),
                    VariableType::Array => value.is_array(),
                    VariableType::Object => value.is_object(),
                    VariableType::Server => value.is_object() || value.is_string(),
                };

                if !valid {
                    errors.push((
                        var.name.clone(),
                        format!("Expected {:?}, got {:?}", var.var_type, value),
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_resolver() {
        let resolver = VariableResolver::new()
            .with_server(ServerContext {
                id: "srv-123".to_string(),
                name: "Production Server".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                password: Some("secret".to_string()),
                key_path: None,
                group: Some("production".to_string()),
                tags: vec!["web".to_string(), "critical".to_string()],
            })
            .with_system_variables();

        let template = "Connecting to {{server.host}}:{{server.port}} as {{server.username}}";
        let resolved = resolver.resolve(template);
        assert!(resolved.contains("192.168.1.100:22"));
        assert!(resolved.contains("admin"));
    }

    #[test]
    fn test_variable_templates() {
        let server_vars = VariableTemplates::server_variables();
        assert!(!server_vars.is_empty());
        assert!(server_vars.iter().any(|v| v.name == "server.host"));

        let system_vars = VariableTemplates::system_variables();
        assert!(!system_vars.is_empty());

        let exec_vars = VariableTemplates::execution_variables();
        assert!(!exec_vars.is_empty());
    }
}
