use crate::domain::{Connection, ConnectionTarget};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("{field} is required")]
    Required { field: &'static str },
    #[error("{field} contains an unsafe value")]
    Unsafe { field: &'static str },
    #[error("port must be between 1 and 65535")]
    Port,
}

pub fn validate_connection(connection: &Connection) -> Result<(), ValidationError> {
    validate_text(&connection.name, "name")?;
    match &connection.target {
        ConnectionTarget::Alias { alias } => validate_alias(alias),
        ConnectionTarget::Endpoint {
            hostname,
            username,
            port,
        } => {
            validate_host(hostname)?;
            if let Some(username) = username {
                validate_username(username)?;
            }
            if *port == 0 {
                return Err(ValidationError::Port);
            }
            Ok(())
        }
    }
}

pub fn validate_alias(value: &str) -> Result<(), ValidationError> {
    validate_token(value, "alias", true)
}
pub fn validate_username(value: &str) -> Result<(), ValidationError> {
    validate_token(value, "username", false)
}
pub fn validate_host(value: &str) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Required { field: "hostname" });
    }
    if value.starts_with('-')
        || value.chars().any(|c| {
            c.is_control() || c.is_whitespace() || matches!(c, ';' | '&' | '|' | '`' | '$')
        })
    {
        return Err(ValidationError::Unsafe { field: "hostname" });
    }
    Ok(())
}

pub fn validate_path(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Required { field });
    }
    if value.starts_with('-') || value.chars().any(|c| c.is_control() || c == '\0') {
        return Err(ValidationError::Unsafe { field });
    }
    Ok(())
}

fn validate_text(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::Required { field });
    }
    if value.chars().any(char::is_control) {
        return Err(ValidationError::Unsafe { field });
    }
    Ok(())
}

fn validate_token(
    value: &str,
    field: &'static str,
    allow_dot: bool,
) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Required { field });
    }
    if value.starts_with('-')
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || (allow_dot && c == '.'))
    {
        return Err(ValidationError::Unsafe { field });
    }
    Ok(())
}

pub fn serialized_has_sensitive_keys(value: &serde_json::Value) -> bool {
    const FORBIDDEN: [&str; 7] = [
        "password",
        "passphrase",
        "private_key",
        "secret",
        "token",
        "vault",
        "credential",
    ];
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, value)| {
            FORBIDDEN
                .iter()
                .any(|word| key.to_ascii_lowercase().contains(word))
                || serialized_has_sensitive_keys(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(serialized_has_sensitive_keys),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_argument_injection() {
        assert!(validate_host("-oProxyCommand=x").is_err());
        assert!(validate_alias("host;id").is_err());
        assert!(validate_path("-target", "local path").is_err());
    }
}
