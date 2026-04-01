-- ============= DevOps事件响应中心数据库迁移 =============
-- 包含事件管理、告警聚合、运行手册、升级策略等所有表

-- ============= 核心事件表 =============

-- 事件主表
CREATE TABLE IF NOT EXISTS incidents (
    id TEXT PRIMARY KEY,
    incident_number TEXT UNIQUE NOT NULL,  -- 格式: INC-YYYYMMDD-XXXX
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    incident_type TEXT NOT NULL,  -- server_down, high_cpu, etc.
    severity TEXT NOT NULL,       -- critical, high, medium, low, info
    status TEXT NOT NULL DEFAULT 'detected',  -- detected, acknowledged, investigating, mitigating, resolved, closed, escalated
    team_id TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    acknowledged_at TIMESTAMP,
    resolved_at TIMESTAMP,
    closed_at TIMESTAMP,
    acknowledged_by TEXT,
    resolved_by TEXT,
    root_cause TEXT,
    impact_summary TEXT,
    affected_servers TEXT,  -- JSON array
    affected_services TEXT, -- JSON array
    assigned_to TEXT,
    escalation_level INTEGER DEFAULT 0,
    parent_incident_id TEXT,
    related_incidents TEXT, -- JSON array
    tags TEXT,              -- JSON array
    metadata TEXT,          -- JSON object
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
);

-- 告警表
CREATE TABLE IF NOT EXISTS alerts (
    id TEXT PRIMARY KEY,
    alert_number TEXT UNIQUE NOT NULL,  -- 格式: ALERT-YYYYMMDD-XXXX
    incident_id TEXT,
    source TEXT NOT NULL,      -- monitoring, prometheus, zabbix, custom
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    team_id TEXT NOT NULL,
    server_id TEXT,
    service_name TEXT,
    metric_name TEXT,
    metric_value REAL,
    threshold REAL,
    status TEXT NOT NULL DEFAULT 'firing',  -- firing, acknowledged, resolved, suppressed, flapping
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    acknowledged_at TIMESTAMP,
    resolved_at TIMESTAMP,
    acknowledged_by TEXT,
    fingerprint TEXT NOT NULL,  -- 用于告警聚合
    aggregation_key TEXT,
    occurrence_count INTEGER DEFAULT 1,
    first_occurrence_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_occurrence_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    raw_data TEXT,  -- JSON
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE SET NULL
);

-- 事件时间线
CREATE TABLE IF NOT EXISTS incident_timeline (
    id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL,
    entry_type TEXT NOT NULL,  -- status_change, severity_change, assignment, escalation, note, action, diagnosis, communication, automation, alert, runbook_executed
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT,  -- JSON
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

-- 诊断结果表
CREATE TABLE IF NOT EXISTS diagnosis_results (
    id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL,
    diagnosis_type TEXT NOT NULL,  -- ai, manual, automated
    findings TEXT NOT NULL,
    confidence_score REAL,
    suggested_actions TEXT,    -- JSON array
    runbook_suggestions TEXT,   -- JSON array of runbook IDs
    related_incidents TEXT,     -- JSON array
    similar_past_incidents TEXT, -- JSON array
    created_by TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_primary BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

-- ============= 运行手册表 =============

-- 运行手册主表
CREATE TABLE IF NOT EXISTS runbooks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    incident_types TEXT,         -- JSON array of applicable incident types
    severity_levels TEXT,      -- JSON array of applicable severity levels
    team_id TEXT NOT NULL,
    is_global BOOLEAN DEFAULT FALSE,
    content TEXT NOT NULL,     -- Markdown format
    steps TEXT,                -- JSON array of structured steps
    automation_script TEXT,    -- Optional automation script
    estimated_duration_minutes INTEGER,
    success_rate REAL DEFAULT 1.0,
    usage_count INTEGER DEFAULT 0,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    tags TEXT,                 -- JSON array
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
);

-- 运行手册执行记录
CREATE TABLE IF NOT EXISTS runbook_executions (
    id TEXT PRIMARY KEY,
    runbook_id TEXT NOT NULL,
    incident_id TEXT NOT NULL,
    executed_by TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, running, paused, completed, failed, cancelled
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    current_step INTEGER DEFAULT 0,
    total_steps INTEGER DEFAULT 0,
    results TEXT,              -- JSON array of step results
    output_log TEXT,
    error_message TEXT,
    FOREIGN KEY (runbook_id) REFERENCES runbooks(id) ON DELETE CASCADE,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

-- ============= 协作处理表 =============

-- 事件参与者
CREATE TABLE IF NOT EXISTS incident_participants (
    id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL,        -- incident_commander, tech_lead, responder, observer, communicator
    joined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    notification_enabled BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE,
    UNIQUE(incident_id, user_id, is_active)
);

-- 事件沟通记录
CREATE TABLE IF NOT EXISTS incident_communications (
    id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL,
    communication_type TEXT NOT NULL,  -- notification, status_update, escalation, resolution, stakeholder_update
    channel TEXT NOT NULL,       -- slack, email, sms, webhook
    recipient TEXT NOT NULL,
    content TEXT NOT NULL,
    sent_by TEXT NOT NULL,
    sent_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL,        -- pending, sent, delivered, failed
    error_message TEXT,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

-- ============= 升级策略表 =============

-- 升级策略
CREATE TABLE IF NOT EXISTS escalation_policies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    team_id TEXT NOT NULL,
    is_default BOOLEAN DEFAULT FALSE,
    rules TEXT NOT NULL,       -- JSON array of escalation rules
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    UNIQUE(team_id, name)
);

-- 升级历史
CREATE TABLE IF NOT EXISTS escalation_history (
    id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL,
    from_level INTEGER NOT NULL,
    to_level INTEGER NOT NULL,
    escalated_by TEXT NOT NULL,
    reason TEXT NOT NULL,
    notified_users TEXT,       -- JSON array
    escalated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

-- 集成配置
CREATE TABLE IF NOT EXISTS integration_configs (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL,
    provider TEXT NOT NULL,    -- pagerduty, opsgenie, slack, teams, webhook, email, sms, discord
    name TEXT NOT NULL,
    config TEXT NOT NULL,    -- JSON provider-specific configuration
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_tested_at TIMESTAMP,
    last_test_status TEXT,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
);

-- ============= 事后复盘表 =============

-- 事后复盘
CREATE TABLE IF NOT EXISTS post_mortems (
    id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    timeline_summary TEXT,
    root_cause_analysis TEXT NOT NULL,
    impact_analysis TEXT,
    resolution_steps TEXT,
    lessons_learned TEXT NOT NULL,
    action_items TEXT,         -- JSON array
    contributors TEXT,         -- JSON array of user IDs
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT DEFAULT 'draft',  -- draft, in_review, approved, published
    created_by TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

-- ============= 事件检测表 =============

-- 检测规则
CREATE TABLE IF NOT EXISTS detection_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    rule_type TEXT NOT NULL,   -- threshold, anomaly, pattern, composite, ml_based
    team_id TEXT NOT NULL,
    conditions TEXT NOT NULL,  -- JSON detection conditions
    severity TEXT NOT NULL,
    auto_create_incident BOOLEAN DEFAULT FALSE,
    auto_assignee TEXT,
    notification_channels TEXT,  -- JSON array
    runbook_id TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (runbook_id) REFERENCES runbooks(id) ON DELETE SET NULL
);

-- ============= 索引优化 =============

-- 事件表索引
CREATE INDEX IF NOT EXISTS idx_incidents_team_id ON incidents(team_id);
CREATE INDEX IF NOT EXISTS idx_incidents_status ON incidents(status);
CREATE INDEX IF NOT EXISTS idx_incidents_severity ON incidents(severity);
CREATE INDEX IF NOT EXISTS idx_incidents_incident_type ON incidents(incident_type);
CREATE INDEX IF NOT EXISTS idx_incidents_created_at ON incidents(created_at);
CREATE INDEX IF NOT EXISTS idx_incidents_assigned_to ON incidents(assigned_to);
CREATE INDEX IF NOT EXISTS idx_incidents_detected_at ON incidents(detected_at);

-- 告警表索引
CREATE INDEX IF NOT EXISTS idx_alerts_team_id ON alerts(team_id);
CREATE INDEX IF NOT EXISTS idx_alerts_incident_id ON alerts(incident_id);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status);
CREATE INDEX IF NOT EXISTS idx_alerts_fingerprint ON alerts(fingerprint);
CREATE INDEX IF NOT EXISTS idx_alerts_created_at ON alerts(created_at);
CREATE INDEX IF NOT EXISTS idx_alerts_server_id ON alerts(server_id);

-- 时间线索引
CREATE INDEX IF NOT EXISTS idx_timeline_incident_id ON incident_timeline(incident_id);
CREATE INDEX IF NOT EXISTS idx_timeline_created_at ON incident_timeline(created_at);

-- 运行手册索引
CREATE INDEX IF NOT EXISTS idx_runbooks_team_id ON runbooks(team_id);
CREATE INDEX IF NOT EXISTS idx_runbooks_is_global ON runbooks(is_global);
CREATE INDEX IF NOT EXISTS idx_runbooks_is_active ON runbooks(is_active);

-- 参与者索引
CREATE INDEX IF NOT EXISTS idx_participants_incident_id ON incident_participants(incident_id);
CREATE INDEX IF NOT EXISTS idx_participants_user_id ON incident_participants(user_id);
CREATE INDEX IF NOT EXISTS idx_participants_is_active ON incident_participants(is_active);

-- 复盘索引
CREATE INDEX IF NOT EXISTS idx_post_mortems_incident_id ON post_mortems(incident_id);
CREATE INDEX IF NOT EXISTS idx_post_mortems_status ON post_mortems(status);

-- 升级历史索引
CREATE INDEX IF NOT EXISTS idx_escalation_history_incident_id ON escalation_history(incident_id);
CREATE INDEX IF NOT EXISTS idx_escalation_history_escalated_at ON escalation_history(escalated_at);

-- 集成索引
CREATE INDEX IF NOT EXISTS idx_integrations_team_id ON integration_configs(team_id);
CREATE INDEX IF NOT EXISTS idx_integrations_provider ON integration_configs(provider);

-- ============= 默认数据 =============

-- 插入默认升级策略规则模板
INSERT OR IGNORE INTO escalation_policies (id, name, team_id, is_default, rules, is_active) VALUES
('esc_policy_default', '标准升级策略', 'default_team', TRUE, '[
  {
    "level": 1,
    "condition": {"condition_type": "no_acknowledgment", "threshold_minutes": 5},
    "notify_users": ["oncall-engineer"],
    "notify_channels": ["slack", "pagerduty"],
    "auto_escalate_after_minutes": 10,
    "require_approval": false
  },
  {
    "level": 2,
    "condition": {"condition_type": "time_based", "threshold_minutes": 15},
    "notify_users": ["sre-manager"],
    "notify_channels": ["slack", "email"],
    "auto_escalate_after_minutes": 30,
    "require_approval": true
  },
  {
    "level": 3,
    "condition": {"condition_type": "time_based", "threshold_minutes": 30},
    "notify_users": ["cto", "vp-engineering"],
    "notify_channels": ["slack", "email", "sms"],
    "require_approval": true
  }
]', TRUE);

-- 插入默认检测规则
INSERT OR IGNORE INTO detection_rules (id, name, description, rule_type, team_id, conditions, severity, auto_create_incident, is_active, created_by, created_at) VALUES
('rule_high_cpu', '高CPU使用率检测', '当CPU使用率超过90%持续5分钟时触发', 'threshold', 'default_team',
 '{"metric": "cpu_usage", "operator": ">", "threshold": 90, "duration_minutes": 5}', 'high', TRUE, TRUE, 'system', CURRENT_TIMESTAMP);

INSERT OR IGNORE INTO detection_rules (id, name, description, rule_type, team_id, conditions, severity, auto_create_incident, is_active, created_by, created_at) VALUES
('rule_high_memory', '高内存使用率检测', '当内存使用率超过85%持续5分钟时触发', 'threshold', 'default_team',
 '{"metric": "memory_usage", "operator": ">", "threshold": 85, "duration_minutes": 5}', 'medium', TRUE, TRUE, 'system', CURRENT_TIMESTAMP);

INSERT OR IGNORE INTO detection_rules (id, name, description, rule_type, team_id, conditions, severity, auto_create_incident, is_active, created_by, created_at) VALUES
('rule_disk_full', '磁盘空间不足检测', '当磁盘使用率超过90%时触发', 'threshold', 'default_team',
 '{"metric": "disk_usage", "operator": ">", "threshold": 90, "duration_minutes": 1}', 'high', TRUE, TRUE, 'system', CURRENT_TIMESTAMP);

INSERT OR IGNORE INTO detection_rules (id, name, description, rule_type, team_id, conditions, severity, auto_create_incident, is_active, created_by, created_at) VALUES
('rule_server_down', '服务器宕机检测', '当服务器无法访问时触发', 'threshold', 'default_team',
 '{"metric": "connectivity", "operator": "==", "threshold": 0, "duration_minutes": 2}', 'critical', TRUE, TRUE, 'system', CURRENT_TIMESTAMP);
