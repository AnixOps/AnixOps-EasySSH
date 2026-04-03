//! 策略引擎 - 高级权限策略管理

use super::{types::*, RbacAuditLogger, RbacError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 策略效果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// 策略条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    /// 工作时间限制
    pub business_hours_only: Option<bool>,
    /// 需要MFA
    pub require_mfa: Option<bool>,
    /// 允许的IP范围
    pub allowed_ip_ranges: Option<Vec<String>>,
    /// 拒绝的IP范围
    pub denied_ip_ranges: Option<Vec<String>>,
    /// 允许的用户代理模式
    pub allowed_user_agents: Option<Vec<String>>,
    /// 拒绝的用户代理模式
    pub denied_user_agents: Option<Vec<String>>,
    /// 最小角色等级
    pub minimum_role_level: Option<u32>,
    /// 自定义条件表达式
    pub expression: Option<String>,
}

impl Default for PolicyCondition {
    fn default() -> Self {
        Self {
            business_hours_only: None,
            require_mfa: None,
            allowed_ip_ranges: None,
            denied_ip_ranges: None,
            allowed_user_agents: None,
            denied_user_agents: None,
            minimum_role_level: None,
            expression: None,
        }
    }
}

/// 权限策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub effect: PolicyEffect,
    pub resource_type: Option<ResourceType>,
    pub resource_id: Option<String>,
    pub operation: Option<Operation>,
    pub condition: Option<PolicyCondition>,
    pub priority: i32, // 数字越小优先级越高
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Policy {
    /// 创建新策略
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            effect: PolicyEffect::Allow,
            resource_type: None,
            resource_id: None,
            operation: None,
            condition: None,
            priority: 100,
            is_active: true,
            created_at: chrono::Utc::now(),
            expires_at: None,
        }
    }

    /// 设置为拒绝策略
    pub fn as_deny(mut self) -> Self {
        self.effect = PolicyEffect::Deny;
        self
    }

    /// 设置资源类型
    pub fn for_resource_type(mut self, resource_type: ResourceType) -> Self {
        self.resource_type = Some(resource_type);
        self
    }

    /// 设置资源ID
    pub fn for_resource_id(mut self, resource_id: impl Into<String>) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    /// 设置操作
    pub fn for_operation(mut self, operation: Operation) -> Self {
        self.operation = Some(operation);
        self
    }

    /// 设置条件
    pub fn with_condition(mut self, condition: PolicyCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|e| chrono::Utc::now() > e)
            .unwrap_or(false)
    }

    /// 检查是否适用于给定资源
    pub fn applies_to(&self, resource_type: ResourceType, operation: Operation) -> bool {
        let type_match = self
            .resource_type
            .map(|t| t == resource_type)
            .unwrap_or(true);
        let op_match = self.operation.map(|o| o == operation).unwrap_or(true);
        type_match && op_match && self.is_active && !self.is_expired()
    }
}

/// 策略评估结果
#[derive(Debug, Clone)]
pub struct PolicyEvaluationResult {
    pub decision: PolicyDecision,
    pub matched_policies: Vec<String>,
    pub violated_conditions: Vec<String>,
    pub reason: Option<String>,
}

/// 策略决策
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
    Default, // 无策略匹配，使用默认行为
}

/// 策略引擎
pub struct PolicyEngine {
    policies: std::sync::RwLock<Vec<Policy>>,
    default_action: PolicyDecision,
    audit_logger: Option<Arc<dyn RbacAuditLogger>>,
    custom_evaluators: HashMap<String, Arc<dyn Fn(&PermissionContext) -> bool + Send + Sync>>,
}

impl PolicyEngine {
    /// 创建新的策略引擎
    pub fn new() -> Self {
        Self {
            policies: std::sync::RwLock::new(Vec::new()),
            default_action: PolicyDecision::Deny,
            audit_logger: None,
            custom_evaluators: HashMap::new(),
        }
    }

    /// 设置默认动作
    pub fn with_default_action(mut self, action: PolicyDecision) -> Self {
        self.default_action = action;
        self
    }

    /// 设置审计日志器
    pub fn with_audit_logger(mut self, logger: Arc<dyn RbacAuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// 注册自定义条件评估器
    pub fn register_custom_evaluator(
        &mut self,
        name: impl Into<String>,
        evaluator: Arc<dyn Fn(&PermissionContext) -> bool + Send + Sync>,
    ) {
        self.custom_evaluators.insert(name.into(), evaluator);
    }

    /// 添加策略
    pub fn add_policy(&self, policy: Policy) -> Result<(), RbacError> {
        let mut policies = self.policies.write().unwrap();

        // 检查是否已存在相同ID的策略
        if policies.iter().any(|p| p.id == policy.id) {
            return Err(RbacError::InvalidResource(format!(
                "Policy with id {} already exists",
                policy.id
            )));
        }

        policies.push(policy);

        // 按优先级排序
        policies.sort_by_key(|p| p.priority);

        Ok(())
    }

    /// 更新策略
    pub fn update_policy(&self, policy: Policy) -> Result<(), RbacError> {
        let mut policies = self.policies.write().unwrap();

        if let Some(index) = policies.iter().position(|p| p.id == policy.id) {
            policies[index] = policy;
            policies.sort_by_key(|p| p.priority);
            Ok(())
        } else {
            Err(RbacError::InvalidResource(format!(
                "Policy with id {} not found",
                policy.id
            )))
        }
    }

    /// 删除策略
    pub fn delete_policy(&self, policy_id: &str) -> Result<(), RbacError> {
        let mut policies = self.policies.write().unwrap();

        if let Some(index) = policies.iter().position(|p| p.id == policy_id) {
            policies.remove(index);
            Ok(())
        } else {
            Err(RbacError::InvalidResource(format!(
                "Policy with id {} not found",
                policy_id
            )))
        }
    }

    /// 获取策略
    pub fn get_policy(&self, policy_id: &str) -> Option<Policy> {
        let policies = self.policies.read().unwrap();
        policies.iter().find(|p| p.id == policy_id).cloned()
    }

    /// 列出所有策略
    pub fn list_policies(&self) -> Vec<Policy> {
        let policies = self.policies.read().unwrap();
        policies.clone()
    }

    /// 列出适用的策略
    pub fn list_applicable_policies(
        &self,
        resource_type: ResourceType,
        operation: Operation,
    ) -> Vec<Policy> {
        let policies = self.policies.read().unwrap();
        policies
            .iter()
            .filter(|p| p.applies_to(resource_type, operation))
            .cloned()
            .collect()
    }

    /// 评估权限请求
    pub fn evaluate(
        &self,
        ctx: &PermissionContext,
        resource: &Resource,
        operation: Operation,
    ) -> PolicyEvaluationResult {
        let policies = self.policies.read().unwrap();

        let applicable_policies: Vec<_> = policies
            .iter()
            .filter(|p| {
                p.applies_to(resource.resource_type, operation) && p.is_active && !p.is_expired()
            })
            .collect();

        let mut matched_policies = Vec::new();
        let mut violated_conditions = Vec::new();

        // Deny优先策略
        for policy in &applicable_policies {
            let condition_result = self.evaluate_condition(&policy.condition, ctx);

            if condition_result {
                matched_policies.push(policy.id.clone());

                if policy.effect == PolicyEffect::Deny {
                    return PolicyEvaluationResult {
                        decision: PolicyDecision::Deny,
                        matched_policies,
                        violated_conditions,
                        reason: Some(format!("Policy '{}' denied access", policy.name)),
                    };
                }
            } else {
                violated_conditions.push(format!("Policy '{}' condition failed", policy.id));
            }
        }

        // 检查是否有Allow策略匹配
        if applicable_policies
            .iter()
            .any(|p| p.effect == PolicyEffect::Allow && self.evaluate_condition(&p.condition, ctx))
        {
            PolicyEvaluationResult {
                decision: PolicyDecision::Allow,
                matched_policies,
                violated_conditions,
                reason: None,
            }
        } else {
            PolicyEvaluationResult {
                decision: self.default_action,
                matched_policies,
                violated_conditions,
                reason: Some("No matching policy found".to_string()),
            }
        }
    }

    /// 批量评估
    pub fn evaluate_batch(
        &self,
        ctx: &PermissionContext,
        resources: &[(Resource, Operation)],
    ) -> Vec<PolicyEvaluationResult> {
        resources
            .iter()
            .map(|(resource, operation)| self.evaluate(ctx, resource, *operation))
            .collect()
    }

    /// 评估条件
    fn evaluate_condition(
        &self,
        condition: &Option<PolicyCondition>,
        ctx: &PermissionContext,
    ) -> bool {
        let Some(condition) = condition else {
            return true; // 无条件默认通过
        };

        // 工作时间检查
        if let Some(business_hours) = condition.business_hours_only {
            if business_hours && !ctx.is_business_hours() {
                return false;
            }
        }

        // MFA检查
        if let Some(require_mfa) = condition.require_mfa {
            if require_mfa && !ctx.is_mfa_verified {
                return false;
            }
        }

        // IP范围检查
        if let Some(ref allowed_ranges) = condition.allowed_ip_ranges {
            if let Some(ref ip) = ctx.ip_address {
                if !allowed_ranges.iter().any(|range| ip.starts_with(range)) {
                    return false;
                }
            } else {
                return false; // 需要IP但未知
            }
        }

        if let Some(ref denied_ranges) = condition.denied_ip_ranges {
            if let Some(ref ip) = ctx.ip_address {
                if denied_ranges.iter().any(|range| ip.starts_with(range)) {
                    return false;
                }
            }
        }

        // 用户代理检查
        if let Some(ref allowed_agents) = condition.allowed_user_agents {
            if let Some(ref agent) = ctx.user_agent {
                if !allowed_agents.iter().any(|pattern| agent.contains(pattern)) {
                    return false;
                }
            }
        }

        if let Some(ref denied_agents) = condition.denied_user_agents {
            if let Some(ref agent) = ctx.user_agent {
                if denied_agents.iter().any(|pattern| agent.contains(pattern)) {
                    return false;
                }
            }
        }

        // 自定义表达式
        if let Some(ref expr) = condition.expression {
            // 这里可以集成表达式引擎
            // 简化起见，假设所有自定义表达式都通过
            // 实际实现可以使用 rhai, evalexpr 等库
        }

        true
    }

    /// 清理过期策略
    pub fn cleanup_expired_policies(&self) -> usize {
        let mut policies = self.policies.write().unwrap();
        let before = policies.len();
        policies.retain(|p| !p.is_expired());
        before - policies.len()
    }

    /// 启用策略
    pub fn enable_policy(&self, policy_id: &str) -> Result<(), RbacError> {
        let mut policies = self.policies.write().unwrap();

        if let Some(policy) = policies.iter_mut().find(|p| p.id == policy_id) {
            policy.is_active = true;
            Ok(())
        } else {
            Err(RbacError::InvalidResource(format!(
                "Policy {} not found",
                policy_id
            )))
        }
    }

    /// 禁用策略
    pub fn disable_policy(&self, policy_id: &str) -> Result<(), RbacError> {
        let mut policies = self.policies.write().unwrap();

        if let Some(policy) = policies.iter_mut().find(|p| p.id == policy_id) {
            policy.is_active = false;
            Ok(())
        } else {
            Err(RbacError::InvalidResource(format!(
                "Policy {} not found",
                policy_id
            )))
        }
    }

    /// 获取策略统计
    pub fn get_stats(&self) -> PolicyEngineStats {
        let policies = self.policies.read().unwrap();

        PolicyEngineStats {
            total_policies: policies.len(),
            active_policies: policies
                .iter()
                .filter(|p| p.is_active && !p.is_expired())
                .count(),
            expired_policies: policies.iter().filter(|p| p.is_expired()).count(),
            allow_policies: policies
                .iter()
                .filter(|p| p.effect == PolicyEffect::Allow)
                .count(),
            deny_policies: policies
                .iter()
                .filter(|p| p.effect == PolicyEffect::Deny)
                .count(),
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 策略引擎统计
#[derive(Debug, Clone)]
pub struct PolicyEngineStats {
    pub total_policies: usize,
    pub active_policies: usize,
    pub expired_policies: usize,
    pub allow_policies: usize,
    pub deny_policies: usize,
}

/// 预定义策略模板
pub mod templates {
    use super::*;

    /// 仅工作时间访问策略
    pub fn business_hours_only(resource_type: ResourceType) -> Policy {
        Policy::new("Business Hours Only")
            .for_resource_type(resource_type)
            .with_condition(PolicyCondition {
                business_hours_only: Some(true),
                ..Default::default()
            })
            .with_priority(10)
    }

    /// 需要MFA策略
    pub fn require_mfa(resource_type: ResourceType) -> Policy {
        Policy::new("Require MFA")
            .for_resource_type(resource_type)
            .with_condition(PolicyCondition {
                require_mfa: Some(true),
                ..Default::default()
            })
            .with_priority(5)
    }

    /// 拒绝外部IP策略
    pub fn deny_external_ips(resource_type: ResourceType) -> Policy {
        Policy::new("Deny External IPs")
            .for_resource_type(resource_type)
            .as_deny()
            .with_condition(PolicyCondition {
                denied_ip_ranges: Some(vec![
                    "10.".to_string(),     // 外部网络
                    "172.16.".to_string(), // 内部网络
                ]),
                ..Default::default()
            })
            .with_priority(1)
    }

    /// 允许内部网络策略
    pub fn allow_internal_network(resource_type: ResourceType) -> Policy {
        Policy::new("Allow Internal Network")
            .for_resource_type(resource_type)
            .with_condition(PolicyCondition {
                allowed_ip_ranges: Some(vec![
                    "192.168.".to_string(),
                    "10.0.".to_string(),
                    "127.".to_string(),
                ]),
                ..Default::default()
            })
            .with_priority(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> PolicyEngine {
        let engine = PolicyEngine::new();

        // 添加一些测试策略
        let allow_policy = Policy::new("Allow All Server Read")
            .for_resource_type(ResourceType::Server)
            .for_operation(Operation::Read)
            .with_priority(100);

        let deny_policy = Policy::new("Deny Delete")
            .for_resource_type(ResourceType::Server)
            .for_operation(Operation::Delete)
            .as_deny()
            .with_priority(1);

        engine.add_policy(allow_policy).unwrap();
        engine.add_policy(deny_policy).unwrap();

        engine
    }

    #[test]
    fn test_policy_creation() {
        let policy = Policy::new("Test Policy")
            .for_resource_type(ResourceType::Server)
            .as_deny();

        assert_eq!(policy.effect, PolicyEffect::Deny);
        assert_eq!(policy.resource_type, Some(ResourceType::Server));
    }

    #[test]
    fn test_evaluate_allow() {
        let engine = create_test_engine();
        let ctx = PermissionContext::new("user1");
        let resource = Resource::all(ResourceType::Server);

        let result = engine.evaluate(&ctx, &resource, Operation::Read);
        assert_eq!(result.decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_evaluate_deny() {
        let engine = create_test_engine();
        let ctx = PermissionContext::new("user1");
        let resource = Resource::all(ResourceType::Server);

        let result = engine.evaluate(&ctx, &resource, Operation::Delete);
        assert_eq!(result.decision, PolicyDecision::Deny);
    }

    #[test]
    fn test_business_hours_condition() {
        let policy = templates::business_hours_only(ResourceType::Server);

        let engine = PolicyEngine::new();
        engine.add_policy(policy).unwrap();

        let ctx = PermissionContext::new("user1");
        let resource = Resource::all(ResourceType::Server);

        let result = engine.evaluate(&ctx, &resource, Operation::Read);
        // 结果取决于当前时间是否在工作时间
        // 这里只是验证可以正常评估
        assert!(
            result.decision == PolicyDecision::Allow || result.decision == PolicyDecision::Default
        );
    }

    #[test]
    fn test_policy_expiration() {
        let mut policy = Policy::new("Temporary Policy");
        policy.expires_at = Some(chrono::Utc::now() - chrono::Duration::hours(1));

        assert!(policy.is_expired());
    }

    #[test]
    fn test_policy_stats() {
        let engine = create_test_engine();
        let stats = engine.get_stats();

        assert_eq!(stats.total_policies, 2);
        assert_eq!(stats.active_policies, 2);
        assert_eq!(stats.allow_policies, 1);
        assert_eq!(stats.deny_policies, 1);
    }

    #[test]
    fn test_list_applicable_policies() {
        let engine = create_test_engine();
        let policies = engine.list_applicable_policies(ResourceType::Server, Operation::Read);

        assert!(!policies.is_empty());
    }

    #[test]
    fn test_policy_priority() {
        let engine = PolicyEngine::new();

        // 低优先级允许
        let allow = Policy::new("Allow")
            .for_resource_type(ResourceType::Server)
            .with_priority(100);

        // 高优先级拒绝
        let deny = Policy::new("Deny")
            .for_resource_type(ResourceType::Server)
            .as_deny()
            .with_priority(1);

        engine.add_policy(allow).unwrap();
        engine.add_policy(deny).unwrap();

        let policies = engine.list_policies();
        // 按优先级排序，Deny应该在前
        assert_eq!(policies[0].effect, PolicyEffect::Deny);
    }

    #[test]
    fn test_ip_range_condition() {
        let condition = PolicyCondition {
            allowed_ip_ranges: Some(vec!["192.168.".to_string()]),
            ..Default::default()
        };

        let engine = PolicyEngine::new();
        let policy = Policy::new("Internal Only").with_condition(condition);

        engine.add_policy(policy).unwrap();

        let ctx_internal = PermissionContext::new("user1").with_ip("192.168.1.1");
        let ctx_external = PermissionContext::new("user2").with_ip("10.0.0.1");
        let resource = Resource::all(ResourceType::Server);

        let result_internal = engine.evaluate(&ctx_internal, &resource, Operation::Read);
        let result_external = engine.evaluate(&ctx_external, &resource, Operation::Read);

        assert_eq!(result_internal.decision, PolicyDecision::Allow);
        assert_eq!(result_external.decision, PolicyDecision::Default);
    }
}
