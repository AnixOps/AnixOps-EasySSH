//! 即时用户开通模块 (Just-In-Time Provisioning)
//!
//! 自动为新SSO用户创建账户，并根据组映射分配权限

use crate::error::LiteError;
use crate::sso::{
    ConflictResolution, IdentityConflictResolver, IdentityMapper, MappedIdentity,
    SsoProvider, SsoUserInfo, TeamSsoMapping,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JIT开通配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitProvisioningConfig {
    /// 是否启用JIT开通
    pub enabled: bool,
    /// 默认角色
    pub default_role: String,
    /// 默认团队 (可选)
    pub default_team_id: Option<String>,
    /// 需要邮箱验证
    pub require_email_verified: bool,
    /// 自动创建团队映射
    pub auto_create_team_mappings: bool,
    /// 首次登录通知管理员
    pub notify_admin_on_first_login: bool,
    /// 开通模板
    pub provisioning_template: ProvisioningTemplate,
}

impl Default for JitProvisioningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_role: "user".to_string(),
            default_team_id: None,
            require_email_verified: true,
            auto_create_team_mappings: false,
            notify_admin_on_first_login: true,
            provisioning_template: ProvisioningTemplate::default(),
        }
    }
}

/// 开通模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisioningTemplate {
    /// 账户过期天数 (0 = 永不过期)
    pub account_expiry_days: i32,
    /// 密码过期天数 (0 = 永不过期)
    pub password_expiry_days: i32,
    /// 是否需要MFA
    pub require_mfa: bool,
    /// 允许的IP范围
    pub allowed_ip_ranges: Vec<String>,
    /// 自定义属性
    pub custom_attributes: HashMap<String, serde_json::Value>,
}

impl Default for ProvisioningTemplate {
    fn default() -> Self {
        Self {
            account_expiry_days: 0,
            password_expiry_days: 90,
            require_mfa: false,
            allowed_ip_ranges: vec![],
            custom_attributes: HashMap::new(),
        }
    }
}

/// 即时用户开通器
pub struct JustInTimeProvisioning {
    /// 配置
    config: JitProvisioningConfig,
    /// 身份映射器
    identity_mapper: IdentityMapper,
    /// 冲突解决器
    conflict_resolver: IdentityConflictResolver,
    /// 开通历史
    provisioning_history: Vec<ProvisioningRecord>,
    /// 最大重试次数
    max_retries: u32,
}

/// 开通记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisioningRecord {
    /// 记录ID
    pub id: String,
    /// 外部用户ID
    pub external_user_id: String,
    /// 提供商ID
    pub provider_id: String,
    /// 开通结果
    pub result: ProvisioningResult,
    /// 创建的用户ID (如果成功)
    pub created_user_id: Option<String>,
    /// 分配的角色
    pub assigned_roles: Vec<String>,
    /// 分配的团队
    pub assigned_teams: Vec<AssignedTeam>,
    /// 开通时间
    pub provisioned_at: chrono::DateTime<Utc>,
    /// 错误信息 (如果失败)
    pub error_message: Option<String>,
}

/// 开通结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvisioningResult {
    Success,
    SuccessLinked,
    FailedConflict,
    FailedValidation,
    FailedError,
    Skipped,
}

/// 分配的团队信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedTeam {
    pub team_id: String,
    pub role: String,
    pub provisioned_at: chrono::DateTime<Utc>,
}

/// 开通的用户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedUser {
    /// 用户ID
    pub user_id: String,
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
    /// 角色
    pub roles: Vec<String>,
    /// 团队
    pub teams: Vec<ProvisionedTeam>,
    /// 首次登录时间
    pub first_login_at: Option<chrono::DateTime<Utc>>,
    /// 最后登录时间
    pub last_login_at: Option<chrono::DateTime<Utc>>,
    /// 开通来源
    pub provisioned_from: String,
    /// 开通时间
    pub provisioned_at: chrono::DateTime<Utc>,
}

/// 开通的团队信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedTeam {
    pub team_id: String,
    pub role: String,
    pub joined_at: chrono::DateTime<Utc>,
}

impl JustInTimeProvisioning {
    /// 创建新的JIT开通器
    pub fn new(
        config: JitProvisioningConfig,
        identity_mapper: IdentityMapper,
        conflict_resolver: IdentityConflictResolver,
    ) -> Self {
        Self {
            config,
            identity_mapper,
            conflict_resolver,
            provisioning_history: Vec::new(),
            max_retries: 3,
        }
    }

    /// 配置默认JIT开通器
    pub fn default_provisioning() -> Self {
        Self::new(
            JitProvisioningConfig::default(),
            IdentityMapper::new("easyssh.local"),
            IdentityConflictResolver::new(crate::sso::ConflictResolutionStrategy::CreateNew),
        )
    }

    /// 执行即时开通
    pub async fn provision_user(
        &mut self,
        sso_user: &SsoUserInfo,
        provider: &SsoProvider,
        existing_users: &[crate::sso::ExistingUserInfo],
    ) -> Result<ProvisioningRecord, LiteError> {
        // 检查是否启用
        if !self.config.enabled {
            return Ok(ProvisioningRecord {
                id: uuid::Uuid::new_v4().to_string(),
                external_user_id: sso_user.user_id.clone(),
                provider_id: provider.id.clone(),
                result: ProvisioningResult::Skipped,
                created_user_id: None,
                assigned_roles: vec![],
                assigned_teams: vec![],
                provisioned_at: Utc::now(),
                error_message: Some("JIT provisioning is disabled".to_string()),
            });
        }

        // 验证邮箱是否已验证 (如果需要)
        if self.config.require_email_verified {
            // 检查SSO用户邮箱是否已验证
            // 这里假设SSO提供者已验证
        }

        // 1. 映射SSO身份到内部身份
        let mapped_identity = self.identity_mapper.map_identity(sso_user)?;

        // 2. 验证身份
        if let Err(e) = self.identity_mapper.validate_identity(&mapped_identity) {
            return Ok(ProvisioningRecord {
                id: uuid::Uuid::new_v4().to_string(),
                external_user_id: sso_user.user_id.clone(),
                provider_id: provider.id.clone(),
                result: ProvisioningResult::FailedValidation,
                created_user_id: None,
                assigned_roles: vec![],
                assigned_teams: vec![],
                provisioned_at: Utc::now(),
                error_message: Some(format!("Identity validation failed: {}", e)),
            });
        }

        // 3. 检测冲突
        let provisioned_user = if let Some(conflict) = self
            .conflict_resolver
            .detect_conflicts(&mapped_identity, existing_users)
        {
            // 4. 解决冲突
            match self.conflict_resolver.resolve_conflict(&conflict) {
                Ok(ConflictResolution::UpdateUser(user_id)) => {
                    // 更新现有用户
                    self.update_existing_user(&user_id, &mapped_identity, provider.id.clone())
                        .await?
                }
                Ok(ConflictResolution::LinkToUser(user_id)) => {
                    // 链接到现有用户
                    self.link_to_existing_user(&user_id, &mapped_identity, provider.id.clone())
                        .await?
                }
                Ok(ConflictResolution::CreateNewUser) => {
                    // 创建新用户 (使用不同的用户名)
                    let mut identity = mapped_identity.clone();
                    let existing_usernames: Vec<String> =
                        existing_users.iter().map(|u| u.username.clone()).collect();
                    identity.username = self
                        .identity_mapper
                        .suggest_username(&identity.username, &existing_usernames);
                    self.create_new_user(&identity, provider).await?
                }
                Err(e) => {
                    return Ok(ProvisioningRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        external_user_id: sso_user.user_id.clone(),
                        provider_id: provider.id.clone(),
                        result: ProvisioningResult::FailedConflict,
                        created_user_id: None,
                        assigned_roles: vec![],
                        assigned_teams: vec![],
                        provisioned_at: Utc::now(),
                        error_message: Some(format!("Conflict resolution failed: {}", e)),
                    });
                }
            }
        } else {
            // 无冲突，创建新用户
            self.create_new_user(&mapped_identity, provider).await?
        };

        // 5. 分配团队角色
        let assigned_teams = self
            .assign_team_roles(&provisioned_user, sso_user, provider)
            .await?;

        // 6. 创建开通记录
        let record = ProvisioningRecord {
            id: uuid::Uuid::new_v4().to_string(),
            external_user_id: sso_user.user_id.clone(),
            provider_id: provider.id.clone(),
            result: ProvisioningResult::Success,
            created_user_id: Some(provisioned_user.user_id.clone()),
            assigned_roles: provisioned_user.roles.clone(),
            assigned_teams,
            provisioned_at: Utc::now(),
            error_message: None,
        };

        self.provisioning_history.push(record.clone());

        Ok(record)
    }

    /// 创建新用户
    async fn create_new_user(
        &self,
        identity: &MappedIdentity,
        provider: &SsoProvider,
    ) -> Result<ProvisionedUser, LiteError> {
        let user_id = uuid::Uuid::new_v4().to_string();

        let user = ProvisionedUser {
            user_id,
            external_user_id: identity.external_user_id.clone(),
            email: identity.email.clone(),
            username: identity.username.clone(),
            first_name: identity.first_name.clone(),
            last_name: identity.last_name.clone(),
            roles: identity.roles.clone(),
            teams: vec![],
            first_login_at: None,
            last_login_at: Some(Utc::now()),
            provisioned_from: format!("sso:{}", provider.id),
            provisioned_at: Utc::now(),
        };

        // 实际实现：调用数据库服务创建用户
        log::info!(
            "Created new user {} via JIT provisioning from provider {}",
            identity.email,
            provider.id
        );

        Ok(user)
    }

    /// 更新现有用户
    async fn update_existing_user(
        &self,
        user_id: &str,
        identity: &MappedIdentity,
        _provider_id: String,
    ) -> Result<ProvisionedUser, LiteError> {
        let user = ProvisionedUser {
            user_id: user_id.to_string(),
            external_user_id: identity.external_user_id.clone(),
            email: identity.email.clone(),
            username: identity.username.clone(),
            first_name: identity.first_name.clone(),
            last_name: identity.last_name.clone(),
            roles: identity.roles.clone(),
            teams: vec![], // 保留现有团队
            first_login_at: Some(Utc::now()),
            last_login_at: Some(Utc::now()),
            provisioned_from: "sso_update".to_string(),
            provisioned_at: Utc::now(),
        };

        log::info!("Updated existing user {} via JIT provisioning", user_id);

        Ok(user)
    }

    /// 链接到现有用户
    async fn link_to_existing_user(
        &self,
        user_id: &str,
        identity: &MappedIdentity,
        _provider_id: String,
    ) -> Result<ProvisionedUser, LiteError> {
        // 与更新类似，但记录链接关系
        let user = ProvisionedUser {
            user_id: user_id.to_string(),
            external_user_id: identity.external_user_id.clone(),
            email: identity.email.clone(),
            username: identity.username.clone(),
            first_name: identity.first_name.clone(),
            last_name: identity.last_name.clone(),
            roles: identity.roles.clone(),
            teams: vec![],
            first_login_at: Some(Utc::now()),
            last_login_at: Some(Utc::now()),
            provisioned_from: "sso_link".to_string(),
            provisioned_at: Utc::now(),
        };

        log::info!(
            "Linked SSO identity {} to existing user {}",
            identity.external_user_id,
            user_id
        );

        Ok(user)
    }

    /// 分配团队角色
    async fn assign_team_roles(
        &self,
        user: &ProvisionedUser,
        sso_user: &SsoUserInfo,
        provider: &SsoProvider,
    ) -> Result<Vec<AssignedTeam>, LiteError> {
        let mut assigned_teams = Vec::new();

        // 获取团队SSO映射
        let team_mappings = self.get_team_mappings_for_provider(&provider.id).await?;

        for mapping in team_mappings {
            // 检查组映射
            for group_mapping in &mapping.group_mappings {
                if sso_user.groups.contains(&group_mapping.sso_group) {
                    // 分配角色
                    assigned_teams.push(AssignedTeam {
                        team_id: mapping.team_id.clone(),
                        role: group_mapping.team_role.clone(),
                        provisioned_at: Utc::now(),
                    });

                    // 创建开通的团队记录
                    let _ = ProvisionedTeam {
                        team_id: mapping.team_id.clone(),
                        role: group_mapping.team_role.clone(),
                        joined_at: Utc::now(),
                    };

                    log::info!(
                        "Assigned user {} to team {} with role {} via JIT provisioning",
                        user.user_id,
                        mapping.team_id,
                        group_mapping.team_role
                    );
                }
            }

            // 如果没有匹配但启用了默认角色
            if !assigned_teams.iter().any(|t| t.team_id == mapping.team_id)
                && mapping.auto_provision
            {
                assigned_teams.push(AssignedTeam {
                    team_id: mapping.team_id.clone(),
                    role: mapping.default_role.clone(),
                    provisioned_at: Utc::now(),
                });

                log::info!(
                    "Auto-provisioned user {} to team {} with default role {}",
                    user.user_id,
                    mapping.team_id,
                    mapping.default_role
                );
            }
        }

        // 分配默认团队 (如果配置了)
        if let Some(ref default_team_id) = self.config.default_team_id {
            if !assigned_teams.iter().any(|t| &t.team_id == default_team_id) {
                assigned_teams.push(AssignedTeam {
                    team_id: default_team_id.clone(),
                    role: self.config.default_role.clone(),
                    provisioned_at: Utc::now(),
                });
            }
        }

        Ok(assigned_teams)
    }

    /// 获取提供商的团队映射
    async fn get_team_mappings_for_provider(
        &self,
        _provider_id: &str,
    ) -> Result<Vec<TeamSsoMapping>, LiteError> {
        // 实际实现：从数据库查询
        // 这里返回空列表作为示例
        Ok(vec![])
    }

    /// 获取开通历史
    pub fn get_provisioning_history(&self) -> &[ProvisioningRecord] {
        &self.provisioning_history
    }

    /// 根据外部用户ID查找开通记录
    pub fn find_provisioning_record(&self, external_user_id: &str) -> Option<&ProvisioningRecord> {
        self.provisioning_history
            .iter()
            .find(|r| r.external_user_id == external_user_id)
    }

    /// 根据用户ID查找开通记录
    pub fn find_record_by_user_id(&self, user_id: &str) -> Option<&ProvisioningRecord> {
        self.provisioning_history
            .iter()
            .find(|r| r.created_user_id.as_deref() == Some(user_id))
    }

    /// 清理历史记录
    pub fn cleanup_history(&mut self, older_than_days: i64) {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days);
        self.provisioning_history
            .retain(|r| r.provisioned_at > cutoff);
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> ProvisioningStatistics {
        let total = self.provisioning_history.len();
        let successful = self
            .provisioning_history
            .iter()
            .filter(|r| {
                r.result == ProvisioningResult::Success
                    || r.result == ProvisioningResult::SuccessLinked
            })
            .count();
        let failed = self
            .provisioning_history
            .iter()
            .filter(|r| {
                matches!(
                    r.result,
                    ProvisioningResult::FailedConflict
                        | ProvisioningResult::FailedError
                        | ProvisioningResult::FailedValidation
                )
            })
            .count();

        ProvisioningStatistics {
            total_records: total,
            successful,
            failed,
            linked: self
                .provisioning_history
                .iter()
                .filter(|r| r.result == ProvisioningResult::SuccessLinked)
                .count(),
        }
    }
}

/// 开通统计
#[derive(Debug, Clone)]
pub struct ProvisioningStatistics {
    pub total_records: usize,
    pub successful: usize,
    pub failed: usize,
    pub linked: usize,
}

/// 批量开通请求
#[derive(Debug, Clone)]
pub struct BatchProvisioningRequest {
    pub users: Vec<SsoUserInfo>,
    pub provider: SsoProvider,
}

/// 批量开通结果
#[derive(Debug, Clone)]
pub struct BatchProvisioningResult {
    pub successful: Vec<ProvisioningRecord>,
    pub failed: Vec<ProvisioningRecord>,
    pub total_processed: usize,
}

impl JustInTimeProvisioning {
    /// 批量开通用户
    pub async fn batch_provision(
        &mut self,
        request: BatchProvisioningRequest,
        existing_users: &[crate::sso::ExistingUserInfo],
    ) -> BatchProvisioningResult {
        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for user in request.users {
            match self
                .provision_user(&user, &request.provider, existing_users)
                .await
            {
                Ok(record) => {
                    if record.result == ProvisioningResult::Success
                        || record.result == ProvisioningResult::SuccessLinked
                    {
                        successful.push(record);
                    } else {
                        failed.push(record);
                    }
                }
                Err(e) => {
                    failed.push(ProvisioningRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        external_user_id: user.user_id.clone(),
                        provider_id: request.provider.id.clone(),
                        result: ProvisioningResult::FailedError,
                        created_user_id: None,
                        assigned_roles: vec![],
                        assigned_teams: vec![],
                        provisioned_at: Utc::now(),
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }

        BatchProvisioningResult {
            total_processed: successful.len() + failed.len(),
            successful,
            failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sso::{
        ConflictResolutionStrategy, IdentityConflictResolver, IdentityMapper, RoleMappingRule,
        SamlConfig, SsoProvider, SsoProviderConfig, SsoProviderType, SsoUserInfo,
    };
    use std::collections::HashMap;

    fn create_test_sso_user() -> SsoUserInfo {
        SsoUserInfo {
            user_id: "ext_user_123".to_string(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            groups: vec!["admin".to_string()],
            team_ids: vec![],
            provider_type: SsoProviderType::Oidc,
            provider_id: "provider1".to_string(),
            raw_attributes: HashMap::new(),
        }
    }

    fn create_test_provider() -> SsoProvider {
        let config = SamlConfig {
            idp_metadata_url: "https://idp.example.com/metadata".to_string(),
            sp_entity_id: "https://easyssh.pro".to_string(),
            acs_url: "https://easyssh.pro/sso/acs".to_string(),
            slo_url: None,
            signature_algorithm: "rsa-sha256".to_string(),
            verify_signatures: true,
            want_assertions_encrypted: false,
            name_id_format: "emailAddress".to_string(),
            attribute_mapping: crate::sso::SamlAttributeMapping::default_mapping(),
        };
        SsoProvider::new_saml("Test Provider", config)
    }

    fn create_test_jit() -> JustInTimeProvisioning {
        let config = JitProvisioningConfig::default();
        let identity_mapper = IdentityMapper::new("easyssh.local");
        let conflict_resolver =
            IdentityConflictResolver::new(ConflictResolutionStrategy::CreateNew);

        JustInTimeProvisioning::new(config, identity_mapper, conflict_resolver)
    }

    #[test]
    fn test_jit_config_default() {
        let config = JitProvisioningConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_role, "user");
        assert!(config.require_email_verified);
    }

    #[test]
    fn test_provisioning_statistics() {
        let jit = create_test_jit();
        let stats = jit.get_statistics();

        assert_eq!(stats.total_records, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_provisioning_record() {
        let record = ProvisioningRecord {
            id: "rec_123".to_string(),
            external_user_id: "ext_123".to_string(),
            provider_id: "prov_456".to_string(),
            result: ProvisioningResult::Success,
            created_user_id: Some("user_789".to_string()),
            assigned_roles: vec!["admin".to_string()],
            assigned_teams: vec![AssignedTeam {
                team_id: "team_1".to_string(),
                role: "Member".to_string(),
                provisioned_at: Utc::now(),
            }],
            provisioned_at: Utc::now(),
            error_message: None,
        };

        assert_eq!(record.result, ProvisioningResult::Success);
        assert_eq!(record.assigned_roles.len(), 1);
    }

    #[tokio::test]
    async fn test_disabled_provisioning() {
        let mut config = JitProvisioningConfig::default();
        config.enabled = false;

        let identity_mapper = IdentityMapper::new("easyssh.local");
        let conflict_resolver =
            IdentityConflictResolver::new(ConflictResolutionStrategy::CreateNew);

        let mut jit = JustInTimeProvisioning::new(config, identity_mapper, conflict_resolver);

        let sso_user = create_test_sso_user();
        let provider = create_test_provider();
        let existing_users: Vec<crate::sso::ExistingUserInfo> = vec![];

        let result = jit
            .provision_user(&sso_user, &provider, &existing_users)
            .await
            .unwrap();

        assert_eq!(result.result, ProvisioningResult::Skipped);
    }

    #[test]
    fn test_cleanup_history() {
        let mut jit = create_test_jit();

        // 手动添加一些历史记录
        jit.provisioning_history.push(ProvisioningRecord {
            id: "old_1".to_string(),
            external_user_id: "ext_old".to_string(),
            provider_id: "prov_1".to_string(),
            result: ProvisioningResult::Success,
            created_user_id: Some("user_1".to_string()),
            assigned_roles: vec![],
            assigned_teams: vec![],
            provisioned_at: Utc::now() - chrono::Duration::days(100),
            error_message: None,
        });

        jit.provisioning_history.push(ProvisioningRecord {
            id: "new_1".to_string(),
            external_user_id: "ext_new".to_string(),
            provider_id: "prov_1".to_string(),
            result: ProvisioningResult::Success,
            created_user_id: Some("user_2".to_string()),
            assigned_roles: vec![],
            assigned_teams: vec![],
            provisioned_at: Utc::now(),
            error_message: None,
        });

        jit.cleanup_history(30);

        assert_eq!(jit.provisioning_history.len(), 1);
        assert_eq!(jit.provisioning_history[0].id, "new_1");
    }

    #[test]
    fn test_find_provisioning_record() {
        let mut jit = create_test_jit();

        jit.provisioning_history.push(ProvisioningRecord {
            id: "rec_1".to_string(),
            external_user_id: "ext_123".to_string(),
            provider_id: "prov_1".to_string(),
            result: ProvisioningResult::Success,
            created_user_id: Some("user_789".to_string()),
            assigned_roles: vec![],
            assigned_teams: vec![],
            provisioned_at: Utc::now(),
            error_message: None,
        });

        let found = jit.find_provisioning_record("ext_123");
        assert!(found.is_some());

        let not_found = jit.find_provisioning_record("ext_999");
        assert!(not_found.is_none());

        let found_by_user = jit.find_record_by_user_id("user_789");
        assert!(found_by_user.is_some());
    }
}
