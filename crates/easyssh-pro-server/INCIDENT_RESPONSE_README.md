# DevOps事件响应中心 (Incident Response Center)

企业级事件检测、响应、协作与复盘系统，参考PagerDuty、OpsGenie、FireHydrant设计。

## 功能模块

### 1. 事件检测 (Event Detection)
- **自动检测**: 基于阈值、异常、模式匹配和ML的自动检测
- **规则引擎**: 灵活的检测规则配置
- **多源集成**: Prometheus、Zabbix、自定义监控源

### 2. 告警聚合 (Alert Aggregation)
- **智能指纹**: 基于告警内容的哈希指纹实现自动聚合
- **告警风暴防护**: 相似告警合并，防止告警泛滥
- **抖动检测**: 自动识别flapping告警

### 3. 事件时间线 (Event Timeline)
- **完整记录**: 事件生命周期全程记录
- **多维条目**: 状态变更、诊断、操作、沟通等
- **可视化**: 清晰的时间线展示

### 4. 自动诊断 (Auto Diagnosis)
- **AI辅助诊断**: 基于历史事件的模式识别
- **根因建议**: 智能根因分析和解决方案推荐
- **相关事件**: 自动关联相似历史事件

### 5. 运行手册 (Runbooks)
- **标准化流程**: 预定义的事件处理SOP
- **自动化执行**: 支持脚本自动化步骤
- **成功率追踪**: 运行手册效果评估

### 6. 协作处理 (Collaboration)
- **角色定义**: 事件指挥官、技术负责人、响应者等
- **实时协作**: 多人同时处理事件
- **沟通追踪**: 所有沟通记录可追溯

### 7. 影响分析 (Impact Analysis)
- **服务器级**: 受影响服务器清单
- **服务级**: 受影响服务和依赖分析
- **业务级**: 业务功能和用户影响评估
- **财务级**: 收入损失和恢复成本估算

### 8. 升级策略 (Escalation)
- **多级升级**: 0-5级升级层次
- **自动升级**: 基于时间和条件的自动升级
- **策略配置**: 灵活的升级策略定义

### 9. 事后复盘 (Post Mortem)
- **自动报告**: 自动生成复盘报告
- **改进建议**: AI生成改进建议
- **行动项**: 可追踪的改进行动项
- **知识沉淀**: 经验总结和知识库更新

### 10. 集成通知 (Integrations)
- **PagerDuty**: 完整PagerDuty集成
- **OpsGenie**: OpsGenie告警管理集成
- **Slack**: Slack通知和协作
- **Microsoft Teams**: Teams通知
- **Webhook**: 通用Webhook支持
- **Email/SMS**: 邮件和短信通知

## API端点

### 事件管理
```
POST   /api/v1/incidents              # 创建事件
GET    /api/v1/incidents              # 查询事件列表
GET    /api/v1/incidents/:id          # 获取事件详情
PUT    /api/v1/incidents/:id          # 更新事件
POST   /api/v1/incidents/:id/acknowledge  # 确认事件
POST   /api/v1/incidents/:id/resolve  # 解决事件
POST   /api/v1/incidents/:id/close    # 关闭事件
GET    /api/v1/incidents/stats       # 事件统计
```

### 告警管理
```
POST   /api/v1/alerts                 # 创建告警
GET    /api/v1/alerts                 # 查询告警
GET    /api/v1/alerts/aggregated      # 聚合告警
GET    /api/v1/alerts/:id             # 获取告警详情
POST   /api/v1/alerts/:id/acknowledge # 确认告警
POST   /api/v1/alerts/:id/resolve     # 解决告警
POST   /api/v1/alerts/:id/suppress    # 抑制告警
```

### 运行手册
```
POST   /api/v1/runbooks               # 创建运行手册
GET    /api/v1/runbooks               # 查询运行手册
GET    /api/v1/runbooks/:id            # 获取运行手册
PUT    /api/v1/runbooks/:id            # 更新运行手册
DELETE /api/v1/runbooks/:id            # 删除运行手册
POST   /api/v1/runbooks/:id/execute    # 执行运行手册
GET    /api/v1/runbooks/:id/executions # 执行历史
GET    /api/v1/runbooks/search         # 搜索运行手册
GET    /api/v1/runbooks/popular        # 热门运行手册
```

### 升级策略
```
POST   /api/v1/escalation-policies         # 创建升级策略
GET    /api/v1/escalation-policies         # 查询策略
GET    /api/v1/escalation-policies/:id      # 获取策略
PUT    /api/v1/escalation-policies/:id      # 更新策略
DELETE /api/v1/escalation-policies/:id      # 删除策略
POST   /api/v1/escalation-policies/:id/test # 测试策略
```

### 集成管理
```
POST   /api/v1/integrations           # 创建集成
GET    /api/v1/integrations           # 查询集成
GET    /api/v1/integrations/:id        # 获取集成详情
PUT    /api/v1/integrations/:id        # 更新集成
DELETE /api/v1/integrations/:id        # 删除集成
POST   /api/v1/integrations/:id/test   # 测试集成
```

### 事后复盘
```
POST   /api/v1/incidents/:id/post-mortem    # 创建复盘
GET    /api/v1/incidents/:id/post-mortem    # 获取复盘
GET    /api/v1/post-mortems                 # 查询复盘列表
GET    /api/v1/post-mortems/:id             # 获取复盘详情
PUT    /api/v1/post-mortems/:id             # 更新复盘
POST   /api/v1/post-mortems/:id/publish     # 发布复盘
GET    /api/v1/post-mortems/:id/report     # 生成报告
GET    /api/v1/post-mortems/:id/suggestions # 获取改进建议
```

### 仪表板和指标
```
GET    /api/v1/metrics                  # 综合指标
GET    /api/v1/dashboard/active-incidents  # 活跃事件仪表板
GET    /api/v1/dashboard/alert-trends   # 告警趋势
```

## 数据结构

### 事件严重程度
- `critical` - P1: 系统完全不可用 (15分钟SLA)
- `high` - P2: 核心功能受损 (1小时SLA)
- `medium` - P3: 部分功能受影响 (4小时SLA)
- `low` - P4: 轻微影响 (24小时SLA)
- `info` - P5: 信息性告警 (1周SLA)

### 事件状态流转
```
detected -> acknowledged -> investigating -> mitigating -> resolved -> closed
                      \                              /
                       ----------------> escalated
```

### 告警指纹算法
告警指纹基于以下字段生成SHA256哈希:
- 告警类型 (alert_type)
- 服务器ID (server_id)
- 服务名 (service_name)
- 指标名 (metric_name)

## 使用示例

### 创建事件
```bash
curl -X POST http://api.example.com/api/v1/incidents \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "生产环境数据库连接池耗尽",
    "description": "应用无法获取数据库连接，大量请求超时",
    "incident_type": "database_error",
    "severity": "critical",
    "team_id": "team-prod",
    "affected_servers": ["db-master-01", "app-server-03"],
    "affected_services": ["payment-service", "user-service"]
  }'
```

### 创建告警
```bash
curl -X POST http://api.example.com/api/v1/alerts \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source": "prometheus",
    "alert_type": "high_cpu",
    "severity": "high",
    "title": "CPU使用率超过90%",
    "description": "服务器 app-server-01 CPU使用率持续超过90%",
    "team_id": "team-prod",
    "server_id": "app-server-01",
    "metric_name": "cpu_usage_percent",
    "metric_value": 95.2,
    "threshold": 90.0
  }'
```

### 执行运行手册
```bash
curl -X POST http://api.example.com/api/v1/runbooks/rbk-123/execute \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "incident_id": "inc-456",
    "executed_by": "user-789"
  }'
```

### 获取聚合告警
```bash
curl http://api.example.com/api/v1/alerts/aggregated \
  -H "Authorization: Bearer $TOKEN"
```

响应示例:
```json
{
  "success": true,
  "data": [
    {
      "aggregation_key": "a1b2c3d4",
      "fingerprint": "a1b2c3d4",
      "alert_count": 12,
      "severity": "high",
      "is_flapping": false,
      "suggested_action": "高频率告警，建议升级为事件并立即处理",
      "first_alert": { ... },
      "latest_alert": { ... }
    }
  ]
}
```

## 核心指标

### MTTR (Mean Time To Resolution)
平均解决时间，按严重程度分类统计

### MTTA (Mean Time To Acknowledge)
平均确认时间，衡量响应速度

### 告警质量
- 假阳性率
- 告警风暴次数
- 聚合效率

### 团队效能
- 事件处理量
- 升级频率
- 复盘完成率

## 架构设计

### 服务分层
```
API Layer (REST/WS)
    |
Service Layer (Business Logic)
    |
Data Layer (SQLx + Redis)
```

### 核心服务
- `IncidentService` - 事件CRUD和生命周期管理
- `RunbookService` - 运行手册管理和执行
- `EscalationService` - 升级策略和通知集成
- `PostMortemService` - 复盘分析和报告生成

### 数据库设计
- 15+核心表覆盖全部功能
- 全面索引优化查询性能
- JSON字段支持灵活数据结构

## 扩展开发

### 添加新的通知集成
1. 在 `IntegrationProvider` enum 添加新类型
2. 在 `EscalationService` 实现发送方法
3. 添加API端点测试方法

### 添加新的检测规则类型
1. 在 `DetectionRuleType` enum 添加新类型
2. 在 `IncidentService` 实现检测逻辑
3. 更新前端配置界面

### 自定义AI诊断
1. 实现新的诊断分析器
2. 注册到 `perform_ai_diagnosis`
3. 扩展诊断结果展示

## 参考实现

参考业界领先的事件管理平台:
- [PagerDuty](https://www.pagerduty.com/)
- [OpsGenie](https://www.atlassian.com/software/opsgenie)
- [FireHydrant](https://firehydrant.io/)
- [Incident.io](https://incident.io/)

## 许可证

MIT License - EasySSH Team
