//! 身份映射模块
//!
//! 提供SSO身份到EasySSH用户身份的映射功能

use crate::error::LiteError;
use crate::sso::{SsoProviderType, SsoUserInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 身份映射器
///
/// 负责将SSO提供者返回的用户属性映射到EasySSH内部用户模型
pub struct IdentityMapper {
    /// 默认域 (用于没有域的用户)
    default_domain: String,
    /// 字段映射配置
    field_mappings: HashMap<String, String>,
    /// 角色映射规则
    role_mappings: Vec<RoleMappingRule>,
}

/// 角色映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMappingRule {
    /// 规则名称
    pub name: String,
    /// 条件字段
    pub condition_field: String,
    /// 条件值 (支持通配符 *)
    pub condition_value: String,
    /// 匹配时分配的角色
    pub role: String,
    /// 优先级 (数字越小优先级越高)
    pub priority: i32,
}

/// 映射后的用户身份
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedIdentity {
    /// 外部用户ID
    pub external_user_id: String,
    /// 邮箱
    pub email: String,
    /// 用户名
    pub username: String,
    /// 名字
    pub first_name: Option<String>,
    /// 姓氏
    pub last_name: Option<String>,
    /// 分配的角色
    pub roles: Vec<String>,
    /// 所属组
    pub groups: Vec<String>,
    /// 额外的属性
    pub attributes: HashMap<String, serde_json::Value>,
    /// 映射来源
    pub mapping_source: String,
}

impl IdentityMapper {
    /// 创建新的身份映射器
    pub fn new(default_domain: &str) -> Self {
        Self {
            default_domain: default_domain.to_string(),
            field_mappings: HashMap::new(),
            role_mappings: Vec::new(),
        }
    }

    /// 添加字段映射
    pub fn add_field_mapping(&mut self, sso_field: &str, internal_field: &str) {
        self.field_mappings
            .insert(sso_field.to_string(), internal_field.to_string());
    }

    /// 添加角色映射规则
    pub fn add_role_mapping(&mut self, rule: RoleMappingRule) {
        self.role_mappings.push(rule);
        // 按优先级排序
        self.role_mappings.sort_by_key(|r| r.priority);
    }

    /// 映射SSO用户信息到内部身份
    pub fn map_identity(&self, sso_user: &SsoUserInfo) -> Result<MappedIdentity, LiteError> {
        let email = self.normalize_email(&sso_user.email)?;
        let username = self.normalize_username(&sso_user.username, &email);

        // 应用角色映射
        let roles = self.map_roles(sso_user);

        let mapped = MappedIdentity {
            external_user_id: sso_user.user_id.clone(),
            email,
            username,
            first_name: sso_user.first_name.clone(),
            last_name: sso_user.last_name.clone(),
            roles,
            groups: sso_user.groups.clone(),
            attributes: sso_user.raw_attributes.clone(),
            mapping_source: format!("{}_{}", sso_user.provider_type, sso_user.provider_id),
        };

        Ok(mapped)
    }

    /// 映射多个SSO用户信息
    pub fn map_identities(&self, sso_users: &[SsoUserInfo]) -> Vec<MappedIdentity> {
        sso_users
            .iter()
            .filter_map(|user| self.map_identity(user).ok())
            .collect()
    }

    /// 提取SSO组到团队角色映射
    pub fn map_groups_to_roles(&self, groups: &[String], team_id: &str) -> Vec<String> {
        let mut roles = Vec::new();

        for group in groups {
            for rule in &self.role_mappings {
                if self.matches_condition(group, &rule.condition_value) {
                    let team_role = format!("{}:{}", team_id, rule.role);
                    if !roles.contains(&team_role) {
                        roles.push(team_role);
                    }
                }
            }
        }

        roles
    }

    /// 验证映射后的身份
    pub fn validate_identity(&self, identity: &MappedIdentity) -> Result<(), LiteError> {
        // 验证邮箱格式
        if !self.is_valid_email(&identity.email) {
            return Err(LiteError::Sso(format!(
                "Invalid email format: {}",
                identity.email
            )));
        }

        // 验证用户名
        if identity.username.is_empty() || identity.username.len() > 64 {
            return Err(LiteError::Sso(
                "Username must be between 1 and 64 characters".to_string(),
            ));
        }

        // 验证必需字段
        if identity.external_user_id.is_empty() {
            return Err(LiteError::Sso("External user ID is required".to_string()));
        }

        Ok(())
    }

    /// 生成用户名建议
    pub fn suggest_username(&self, base_name: &str, existing_usernames: &[String]) -> String {
        let normalized = self.normalize_username(base_name, base_name);

        if !existing_usernames.contains(&normalized) {
            return normalized;
        }

        // 尝试添加数字后缀
        for i in 1..=100 {
            let suggestion = format!("{}{}", normalized, i);
            if !existing_usernames.contains(&suggestion) {
                return suggestion;
            }
        }

        // 添加随机后缀
        format!("{}_{}", normalized, crate::sso::generate_secure_random(6))
    }

    /// 标准化邮箱地址
    fn normalize_email(&self, email: &str) -> Result<String, LiteError> {
        let email = email.trim().to_lowercase();

        if !self.is_valid_email(&email) {
            return Err(LiteError::Sso(format!("Invalid email: {}", email)));
        }

        Ok(email)
    }

    /// 标准化用户名
    fn normalize_username(&self, username: &str, fallback: &str) -> String {
        let normalized: String = username
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
            .take(64)
            .collect();

        if normalized.is_empty() {
            // 使用fallback生成用户名
            return fallback
                .split('@')
                .next()
                .unwrap_or("user")
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                .take(64)
                .collect();
        }

        normalized
    }

    /// 映射角色
    fn map_roles(&self, sso_user: &SsoUserInfo) -> Vec<String> {
        let mut roles = Vec::new();

        for rule in &self.role_mappings {
            let field_value = match rule.condition_field.as_str() {
                "groups" => sso_user.groups.join(","),
                "email" => sso_user.email.clone(),
                "user_id" => sso_user.user_id.clone(),
                _ => {
                    // 从raw_attributes获取
                    sso_user
                        .raw_attributes
                        .get(&rule.condition_field)
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default()
                }
            };

            if self.matches_condition(&field_value, &rule.condition_value) {
                if !roles.contains(&rule.role) {
                    roles.push(rule.role.clone());
                }
            }
        }

        // 如果没有匹配任何角色，添加默认角色
        if roles.is_empty() {
            roles.push("user".to_string());
        }

        roles
    }

    /// 检查条件是否匹配 (支持通配符 *)
    fn matches_condition(&self, value: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            // 使用简单的通配符匹配
            let regex_pattern = pattern.replace("*", ".*");
            if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return re.is_match(value);
            }
        }

        value == pattern
    }

    /// 验证邮箱格式
    fn is_valid_email(&self, email: &str) -> bool {
        // 简单的邮箱验证正则
        let email_regex = regex::Regex::new(
            r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
        );
        match email_regex {
            Ok(re) => re.is_match(email),
            Err(_) => false,
        }
    }
}

/// 身份冲突解决器
pub struct IdentityConflictResolver {
    /// 冲突解决策略
    strategy: ConflictResolutionStrategy,
}

/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolutionStrategy {
    /// 拒绝 (返回错误)
    Reject,
    /// 更新现有用户
    Update,
    /// 创建新用户 (添加数字后缀)
    CreateNew,
    /// 链接到现有用户
    Link,
}

/// 身份冲突
#[derive(Debug, Clone)]
pub struct IdentityConflict {
    /// 外部身份
    pub external_identity: MappedIdentity,
    /// 匹配的现有用户
    pub existing_matches: Vec<ExistingUserMatch>,
    /// 冲突类型
    pub conflict_type: ConflictType,
}

/// 现有用户匹配
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ExistingUserMatch {
    /// 用户ID
    pub user_id: String,
    /// 匹配字段
    pub matched_field: String,
    /// 匹配值
    pub matched_value: String,
}

/// 带有置信度的匹配信息 (用于解决冲突)
#[derive(Debug, Clone)]
pub struct ExistingUserMatchWithConfidence {
    pub user_match: ExistingUserMatch,
    /// 匹配置信度 (0-1)
    pub confidence: f64,
}

/// 冲突类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// 邮箱已存在
    EmailExists,
    /// 用户名已存在
    UsernameExists,
    /// 外部ID已存在
    ExternalIdExists,
    /// 多个匹配
    MultipleMatches,
}

impl IdentityConflictResolver {
    /// 创建新的冲突解决器
    pub fn new(strategy: ConflictResolutionStrategy) -> Self {
        Self { strategy }
    }

    /// 解决身份冲突
    pub fn resolve_conflict(
        &self,
        conflict: &IdentityConflict,
    ) -> Result<ConflictResolution, LiteError> {
        match self.strategy {
            ConflictResolutionStrategy::Reject => {
                Err(LiteError::Sso(format!(
                    "Identity conflict detected: {:?}",
                    conflict.conflict_type
                )))
            }
            ConflictResolutionStrategy::Update => {
                // 选择第一个匹配进行更新
                let best_match = conflict
                    .existing_matches
                    .first()
                    .ok_or_else(|| LiteError::Sso("No existing match found".to_string()))?;

                Ok(ConflictResolution::UpdateUser(best_match.user_id.clone()))
            }
            ConflictResolutionStrategy::CreateNew => {
                Ok(ConflictResolution::CreateNewUser)
            }
            ConflictResolutionStrategy::Link => {
                // 选择第一个匹配进行链接
                let best_match = conflict
                    .existing_matches
                    .first()
                    .ok_or_else(|| LiteError::Sso("No existing match found".to_string()))?;

                Ok(ConflictResolution::LinkToUser(best_match.user_id.clone()))
            }
        }
    }

    /// 检测身份冲突
    pub fn detect_conflicts(
        &self,
        identity: &MappedIdentity,
        existing_users: &[ExistingUserInfo],
    ) -> Option<IdentityConflict> {
        let mut matches = Vec::new();

        // 检查邮箱匹配
        for user in existing_users {
            if user.email == identity.email {
                matches.push(ExistingUserMatch {
                    user_id: user.user_id.clone(),
                    matched_field: "email".to_string(),
                    matched_value: identity.email.clone(),
                });
            }

            // 检查外部ID匹配
            if let Some(ref ext_id) = user.external_user_id {
                if ext_id == &identity.external_user_id {
                    matches.push(ExistingUserMatch {
                        user_id: user.user_id.clone(),
                        matched_field: "external_user_id".to_string(),
                        matched_value: identity.external_user_id.clone(),
                    });
                }
            }

            // 检查用户名匹配
            if user.username == identity.username {
                matches.push(ExistingUserMatch {
                    user_id: user.user_id.clone(),
                    matched_field: "username".to_string(),
                    matched_value: identity.username.clone(),
                });
            }
        }

        if matches.is_empty() {
            return None;
        }

        // 去重
        let unique_matches: Vec<_> = matches
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let conflict_type = if unique_matches.len() > 1 {
            ConflictType::MultipleMatches
        } else if unique_matches[0].matched_field == "email" {
            ConflictType::EmailExists
        } else if unique_matches[0].matched_field == "external_user_id" {
            ConflictType::ExternalIdExists
        } else {
            ConflictType::UsernameExists
        };

        Some(IdentityConflict {
            external_identity: identity.clone(),
            existing_matches: unique_matches,
            conflict_type,
        })
    }
}

/// 冲突解决结果
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    /// 更新现有用户
    UpdateUser(String),
    /// 创建新用户
    CreateNewUser,
    /// 链接到现有用户
    LinkToUser(String),
}

/// 现有用户信息
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ExistingUserInfo {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub external_user_id: Option<String>,
}

/// 默认身份映射配置
pub fn default_identity_mapping() -> IdentityMapper {
    let mut mapper = IdentityMapper::new("easyssh.local");

    // 添加默认角色映射规则
    mapper.add_role_mapping(RoleMappingRule {
        name: "admin_group".to_string(),
        condition_field: "groups".to_string(),
        condition_value: "*admin*".to_string(),
        role: "admin".to_string(),
        priority: 1,
    });

    mapper.add_role_mapping(RoleMappingRule {
        name: "developers_group".to_string(),
        condition_field: "groups".to_string(),
        condition_value: "*developers*".to_string(),
        role: "developer".to_string(),
        priority: 2,
    });

    mapper.add_role_mapping(RoleMappingRule {
        name: "default_user".to_string(),
        condition_field: "*".to_string(),
        condition_value: "*".to_string(),
        role: "user".to_string(),
        priority: 100,
    });

    mapper
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sso::SsoUserInfo;

    fn create_test_sso_user() -> SsoUserInfo {
        SsoUserInfo {
            user_id: "ext_user_123".to_string(),
            email: "test.user@example.com".to_string(),
            username: "testuser".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            groups: vec!["developers".to_string(), "users".to_string()],
            team_ids: vec![],
            provider_type: SsoProviderType::Oidc,
            provider_id: "provider1".to_string(),
            raw_attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_identity_mapping() {
        let sso_user = create_test_sso_user();
        let mapper = default_identity_mapping();

        let mapped = mapper.map_identity(&sso_user).unwrap();

        assert_eq!(mapped.external_user_id, "ext_user_123");
        assert_eq!(mapped.email, "test.user@example.com");
        assert_eq!(mapped.username, "testuser");
        assert!(mapped.roles.contains(&"developer".to_string()));
    }

    #[test]
    fn test_group_role_mapping() {
        let mapper = default_identity_mapping();

        let roles = mapper.map_groups_to_roles(
            &vec!["admin".to_string(), "users".to_string()],
            "team1",
        );

        assert!(roles.contains(&"team1:admin".to_string()));
    }

    #[test]
    fn test_wildcard_matching() {
        let mapper = IdentityMapper::new("easyssh.local");

        assert!(mapper.matches_condition("admin_group", "*admin*"));
        assert!(mapper.matches_condition("superadmin", "*admin*"));
        assert!(!mapper.matches_condition("user", "*admin*"));
        assert!(mapper.matches_condition("anything", "*"));
    }

    #[test]
    fn test_username_normalization() {
        let mapper = IdentityMapper::new("easyssh.local");

        let username1 = mapper.normalize_username("Test.User", "fallback");
        assert_eq!(username1, "test.user");

        let username2 = mapper.normalize_username("User@Domain", "fallback");
        assert_eq!(username2, "userdomain");

        let username3 = mapper.normalize_username("", "email@example.com");
        assert_eq!(username3, "email");
    }

    #[test]
    fn test_conflict_detection() {
        let resolver = IdentityConflictResolver::new(ConflictResolutionStrategy::Update);

        let identity = MappedIdentity {
            external_user_id: "ext_123".to_string(),
            email: "existing@example.com".to_string(),
            username: "existinguser".to_string(),
            first_name: Some("Existing".to_string()),
            last_name: Some("User".to_string()),
            roles: vec!["user".to_string()],
            groups: vec![],
            attributes: HashMap::new(),
            mapping_source: "test".to_string(),
        };

        let existing = vec![
            ExistingUserInfo {
                user_id: "user_456".to_string(),
                email: "existing@example.com".to_string(),
                username: "different".to_string(),
                external_user_id: None,
            },
        ];

        let conflict = resolver.detect_conflicts(&identity, &existing);
        assert!(conflict.is_some());
        assert_eq!(conflict.unwrap().conflict_type, ConflictType::EmailExists);
    }

    #[test]
    fn test_conflict_resolution() {
        let resolver = IdentityConflictResolver::new(ConflictResolutionStrategy::Update);

        let conflict = IdentityConflict {
            external_identity: MappedIdentity {
                external_user_id: "ext_123".to_string(),
                email: "test@example.com".to_string(),
                username: "testuser".to_string(),
                first_name: None,
                last_name: None,
                roles: vec![],
                groups: vec![],
                attributes: HashMap::new(),
                mapping_source: "test".to_string(),
            },
            existing_matches: vec![
                ExistingUserMatch {
                    user_id: "user_456".to_string(),
                    matched_field: "email".to_string(),
                    matched_value: "test@example.com".to_string(),
                },
            ],
            conflict_type: ConflictType::EmailExists,
        };

        let resolution = resolver.resolve_conflict(&conflict).unwrap();
        match resolution {
            ConflictResolution::UpdateUser(user_id) => {
                assert_eq!(user_id, "user_456");
            }
            _ => panic!("Expected UpdateUser resolution"),
        }
    }

    #[test]
    fn test_suggest_username() {
        let mapper = IdentityMapper::new("easyssh.local");

        let existing = vec!["john".to_string(), "john1".to_string()];
        let suggestion = mapper.suggest_username("john", &existing);
        assert_eq!(suggestion, "john2");

        let existing2 = vec!["test".to_string()];
        let suggestion2 = mapper.suggest_username("test", &existing2);
        assert_eq!(suggestion2, "test1");
    }

    #[test]
    fn test_validate_identity() {
        let mapper = IdentityMapper::new("easyssh.local");

        let valid_identity = MappedIdentity {
            external_user_id: "ext_123".to_string(),
            email: "valid@example.com".to_string(),
            username: "validuser".to_string(),
            first_name: Some("Valid".to_string()),
            last_name: Some("User".to_string()),
            roles: vec!["user".to_string()],
            groups: vec![],
            attributes: HashMap::new(),
            mapping_source: "test".to_string(),
        };

        assert!(mapper.validate_identity(&valid_identity).is_ok());

        let invalid_identity = MappedIdentity {
            external_user_id: "ext_123".to_string(),
            email: "invalid-email".to_string(),
            username: "validuser".to_string(),
            first_name: None,
            last_name: None,
            roles: vec![],
            groups: vec![],
            attributes: HashMap::new(),
            mapping_source: "test".to_string(),
        };

        assert!(mapper.validate_identity(&invalid_identity).is_err());
    }
}
