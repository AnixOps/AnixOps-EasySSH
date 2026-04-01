# EasySSH 文档索引

> 本文档索引包含所有项目文档，按类别组织。最后更新：2026-04-01

---

## 快速导航

| 类别 | 描述 | 位置 |
|------|------|------|
| [📋 产品规划](#产品规划) | 版本规划、竞品分析 | `docs/` |
| [🏗️ 架构设计](#架构设计) | 系统架构、API设计、数据流 | `docs/architecture/` |
| [👨‍💻 开发者指南](#开发者指南) | 开发环境、调试、测试 | `docs/developers/` |
| [📊 标准规范](#标准规范) | 代码质量、UI/UX标准 | `docs/standards/` |
| [🔒 安全文档](#安全文档) | 安全审计、漏洞修复 | `docs/security/` |
| [🚀 部署运维](#部署运维) | CI/CD、部署指南、监控 | `docs/deployment/` |
| [📈 分析报告](#分析报告) | 性能分析、代码统计 | `docs/analysis/` |
| [🔧 功能实现](#功能实现) | 具体功能实现文档 | `docs/features/` |
| [🌐 产品文档](#产品文档) | 用户文档、官网 | `docs-product/` |

---

## 产品规划

### 版本规划文档

| 文档 | 描述 | 路径 |
|------|------|------|
| [竞品分析](competitor-analysis.md) | SSH客户端痛点、优点、警示分析 | `docs/competitor-analysis.md` |
| [Lite版本规划](easyssh-lite-planning.md) | Lite版完整功能规格 | `docs/easyssh-lite-planning.md` |
| [Standard版本规划](easyssh-standard-planning.md) | Standard版完整功能规格 | `docs/easyssh-standard-planning.md` |
| [Pro版本规划](easyssh-pro-planning.md) | Pro版完整功能规格 | `docs/easyssh-pro-planning.md` |

### 项目总览

| 文档 | 描述 | 路径 |
|------|------|------|
| [CLAUDE.md](../CLAUDE.md) | 项目总览、技术栈、功能矩阵 | `CLAUDE.md` |
| [PROJECT_SUMMARY.md](../PROJECT_SUMMARY.md) | 项目摘要、里程碑、KPI | `PROJECT_SUMMARY.md` |
| [PROJECT_DASHBOARD.md](../PROJECT_DASHBOARD.md) | 项目仪表板、任务追踪 | `PROJECT_DASHBOARD.md` |
| [FINAL_REPORT.md](../FINAL_REPORT.md) | 项目最终报告 | `FINAL_REPORT.md` |

---

## 架构设计

### 核心架构

| 文档 | 描述 | 路径 |
|------|------|------|
| [整体架构](architecture/overall-architecture.md) | Monorepo结构、依赖关系 | `docs/architecture/overall-architecture.md` |
| [系统架构](architecture/system-architecture.md) | 详细系统设计 | `docs/architecture/system-architecture.md` |
| [API设计](architecture/api-design.md) | API设计规范 | `docs/architecture/api-design.md` |
| [数据流](architecture/data-flow.md) | 数据流设计 | `docs/architecture/data-flow.md` |
| [部署架构](architecture/deployment.md) | 部署架构设计 | `docs/architecture/deployment.md` |
| [Termius风格重构](architecture/termius-inspired-redesign.md) | 全平台工作区重构方案 | `docs/architecture/termius-inspired-redesign.md` |

### 技术架构

| 文档 | 描述 | 路径 |
|------|------|------|
| [ARCHITECTURE.md](../ARCHITECTURE.md) | 技术架构总览 | `ARCHITECTURE.md` |
| [API_GUIDE.md](../API_GUIDE.md) | API使用指南 | `API_GUIDE.md` |
| [INTEGRATION_PLAN.md](../INTEGRATION_PLAN.md) | 系统集成计划 | `INTEGRATION_PLAN.md` |

---

## 开发者指南

### 开发环境

| 文档 | 描述 | 路径 |
|------|------|------|
| [开发环境设置](developers/SETUP.md) | 开发环境配置指南 | `docs/developers/SETUP.md` |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | 贡献指南、代码规范 | `CONTRIBUTING.md` |
| [TESTING_SETUP.md](../TESTING_SETUP.md) | 测试环境配置 | `TESTING_SETUP.md` |

### 调试与测试

| 文档 | 描述 | 路径 |
|------|------|------|
| [调试指南](developers/DEBUGGING.md) | 调试技巧、工具使用 | `docs/developers/DEBUGGING.md` |
| [测试指南](developers/TESTING.md) | 测试策略、用例编写 | `docs/developers/TESTING.md` |
| [性能分析](developers/PROFILING.md) | 性能分析工具、方法 | `docs/developers/PROFILING.md` |
| [故障排查](developers/TROUBLESHOOTING.md) | 常见问题解决 | `docs/developers/TROUBLESHOOTING.md` |

### 开发工具

| 文档 | 描述 | 路径 |
|------|------|------|
| [AI助手实现](AI_ASSISTANT_IMPLEMENTATION.md) | AI辅助开发集成 | `docs/AI_ASSISTANT_IMPLEMENTATION.md` |
| [AUTONOMOUS_DEV.md](AUTONOMOUS_DEV.md) | 自主开发模式 | `docs/AUTONOMOUS_DEV.md` |
| [debug-interface.md](standards/debug-interface.md) | Debug接口规范 | `docs/standards/debug-interface.md` |

---

## 标准规范

### 代码质量

| 文档 | 描述 | 路径 |
|------|------|------|
| [代码质量标准](standards/code-quality.md) | Rust/TypeScript编码规范 | `docs/standards/code-quality.md` |
| [UI/UX自动化](standards/ui-ux-automation.md) | AI辅助设计、视觉回归测试 | `docs/standards/ui-ux-automation.md` |
| [代码质量报告](../code-quality-report.md) | 代码质量分析报告 | `code-quality-report.md` |
| [复杂度报告](../complexity-report.md) | 代码复杂度分析 | `complexity-report.md` |

---

## 安全文档

### 安全审计

| 文档 | 描述 | 路径 |
|------|------|------|
| [安全审计报告](../SECURITY_AUDIT_REPORT.md) | 安全审计详细报告 | `SECURITY_AUDIT_REPORT.md` |
| [安全审计修复报告](../SECURITY_AUDIT_FIX_REPORT.md) | 安全漏洞修复记录 | `SECURITY_AUDIT_FIX_REPORT.md` |
| [安全审计完成报告](../SECURITY_AUDIT_COMPLETE_2026-04-01.md) | 2026-04-01审计完成报告 | `SECURITY_AUDIT_COMPLETE_2026-04-01.md` |
| [安全补丁指南](../SECURITY_PATCH_GUIDE.md) | 安全补丁应用指南 | `SECURITY_PATCH_GUIDE.md` |

---

## 部署运维

### CI/CD

| 文档 | 描述 | 路径 |
|------|------|------|
| [CI/CD配置](../CI_CD_CONFIGURATION.md) | CI/CD详细配置 | `CI_CD_CONFIGURATION.md` |
| [CI/CD摘要](../CI_CD_SUMMARY.md) | CI/CD配置摘要 | `CI_CD_SUMMARY.md` |
| [DEPLOYMENT.md](../DEPLOYMENT.md) | 部署指南 | `DEPLOYMENT.md` |
| [BUILD_AUTOMATION_REPORT.md](../BUILD_AUTOMATION_REPORT.md) | 构建自动化报告 | `BUILD_AUTOMATION_REPORT.md` |
| [RELEASE_CHECKLIST.md](../RELEASE_CHECKLIST.md) | 发布检查清单 | `RELEASE_CHECKLIST.md` |
| [RELEASE_NOTES.md](../RELEASE_NOTES.md) | 发布说明 | `RELEASE_NOTES.md` |

### 自动更新

| 文档 | 描述 | 路径 |
|------|------|------|
| [auto-update-readme.md](auto-update-readme.md) | 自动更新功能说明 | `docs/auto-update-readme.md` |
| [auto-update-api.md](auto-update-api.md) | 自动更新API文档 | `docs/auto-update-api.md` |
| [auto-update-deployment.md](auto-update-deployment.md) | 自动更新部署指南 | `docs/auto-update-deployment.md` |
| [auto-update-implementation-summary.md](auto-update-implementation-summary.md) | 自动更新实现总结 | `docs/auto-update-implementation-summary.md` |

### 监控与日志

| 文档 | 描述 | 路径 |
|------|------|------|
| [遥测分析](telemetry-analytics.md) | 遥测数据分析 | `docs/telemetry-analytics.md` |
| [遥测实现总结](telemetry-implementation-summary.md) | 遥测功能实现 | `docs/telemetry-implementation-summary.md` |
| [LOG_MONITOR_IMPLEMENTATION.md](../LOG_MONITOR_IMPLEMENTATION.md) | 日志监控实现 | `LOG_MONITOR_IMPLEMENTATION.md` |
| [MONITORING_DASHBOARD_IMPLEMENTATION.md](../MONITORING_DASHBOARD_IMPLEMENTATION.md) | 监控仪表板实现 | `MONITORING_DASHBOARD_IMPLEMENTATION.md` |

---

## 分析报告

### 代码分析

| 文档 | 描述 | 路径 |
|------|------|------|
| [代码统计](../CODE_STATISTICS.md) | 代码行数、文件统计 | `CODE_STATISTICS.md` |
| [STATISTICS.md](../STATISTICS.md) | 项目统计报告 | `STATISTICS.md` |
| [依赖优化报告](dependency-optimization-report.md) | 依赖优化分析 | `docs/dependency-optimization-report.md` |
| [依赖清理总结](DEPENDENCY_CLEANUP_SUMMARY.md) | 依赖清理结果 | `docs/DEPENDENCY_CLEANUP_SUMMARY.md` |
| [依赖优化最终报告](DEPENDENCY_OPTIMIZATION_FINAL.md) | 依赖优化最终报告 | `docs/DEPENDENCY_OPTIMIZATION_FINAL.md` |
| [内存分析报告](../memory-analysis-report.md) | 内存使用分析 | `memory-analysis-report.md` |
| [启动优化报告](startup-optimization-report.md) | 启动性能优化 | `docs/startup-optimization-report.md` |
| [Standard优化报告](../STANDARD_OPTIMIZATION_REPORT.md) | Standard版优化 | `STANDARD_OPTIMIZATION_REPORT.md` |
| [CROSS_PLATFORM_COMPATIBILITY_REPORT.md](CROSS_PLATFORM_COMPATIBILITY_REPORT.md) | 跨平台兼容性报告 | `docs/CROSS_PLATFORM_COMPATIBILITY_REPORT.md` |

### 依赖分析

| 文档 | 描述 | 路径 |
|------|------|------|
| [依赖分析报告](dependency-analysis/dependency-report.md) | 依赖关系分析 | `docs/dependency-analysis/dependency-report.md` |
| [依赖图DOT](dependency-analysis/dependency-graph.dot) | 依赖图源文件 | `docs/dependency-analysis/dependency-graph.dot` |
| [依赖图HTML](dependency-analysis/dependency-graph.svg.html) | 可视化依赖图 | `docs/dependency-analysis/dependency-graph.svg.html` |

### 测试报告

| 文档 | 描述 | 路径 |
|------|------|------|
| [测试覆盖报告](../TEST_COVERAGE_REPORT_AGENT15.md) | 测试覆盖率分析 | `TEST_COVERAGE_REPORT_AGENT15.md` |
| [回归测试报告](../REGRESSION_TEST_REPORT.md) | 回归测试结果 | `REGRESSION_TEST_REPORT.md` |
| [debug-test-report.md](../debug-test-report.md) | Debug测试报告 | `debug-test-report.md` |
| [DOCUMENTATION_REPORT.md](../DOCUMENTATION_REPORT.md) | 文档质量报告 | `DOCUMENTATION_REPORT.md` |

---

## 功能实现

### 核心功能

| 文档 | 描述 | 路径 |
|------|------|------|
| [SPLIT_LAYOUT.md](SPLIT_LAYOUT.md) | 分屏布局系统 | `docs/SPLIT_LAYOUT.md` |
| [TERMINAL_INTEGRATION.md](../TERMINAL_INTEGRATION.md) | 终端集成方案 | `TERMINAL_INTEGRATION.md` |
| [DATABASE_CLIENT_IMPLEMENTATION.md](../DATABASE_CLIENT_IMPLEMENTATION.md) | 数据库客户端实现 | `DATABASE_CLIENT_IMPLEMENTATION.md` |
| [IMPORT_EXPORT_IMPLEMENTATION.md](../IMPORT_EXPORT_IMPLEMENTATION.md) | 导入导出实现 | `IMPORT_EXPORT_IMPLEMENTATION.md` |
| [IMPORT_EXPORT_FORMATS.md](../IMPORT_EXPORT_FORMATS.md) | 导入导出格式规范 | `IMPORT_EXPORT_FORMATS.md` |
| [SYNC_SYSTEM.md](../SYNC_SYSTEM.md) | 同步系统实现 | `SYNC_SYSTEM.md` |
| [BACKUP_IMPLEMENTATION_SUMMARY.md](../BACKUP_IMPLEMENTATION_SUMMARY.md) | 备份功能实现 | `BACKUP_IMPLEMENTATION_SUMMARY.md` |

### 高级功能

| 文档 | 描述 | 路径 |
|------|------|------|
| [WORKFLOW_SYSTEM_SUMMARY.md](../WORKFLOW_SYSTEM_SUMMARY.md) | 工作流系统总结 | `WORKFLOW_SYSTEM_SUMMARY.md` |
| [SESSION_RECORDING.md](SESSION_RECORDING.md) | 会话录制功能 | `docs/SESSION_RECORDING.md` |
| [RECORDING_IMPLEMENTATION_REPORT.md](../RECORDING_IMPLEMENTATION_REPORT.md) | 录制功能实现报告 | `RECORDING_IMPLEMENTATION_REPORT.md` |
| [REMOTE_DESKTOP_IMPLEMENTATION.md](../REMOTE_DESKTOP_IMPLEMENTATION.md) | 远程桌面实现 | `REMOTE_DESKTOP_IMPLEMENTATION.md` |
| [KUBERNETES_IMPLEMENTATION.md](../KUBERNETES_IMPLEMENTATION.md) | Kubernetes支持实现 | `KUBERNETES_IMPLEMENTATION.md` |
| [ENTERPRISE_VAULT.md](enterprise-vault.md) | 企业密钥库 | `docs/enterprise-vault.md` |

### 国际化

| 文档 | 描述 | 路径 |
|------|------|------|
| [i18n-guide.md](i18n-guide.md) | 国际化开发指南 | `docs/i18n-guide.md` |

### 特性开关

| 文档 | 描述 | 路径 |
|------|------|------|
| [FEATURE_FLAG_FIXES_REPORT.md](../FEATURE_FLAG_FIXES_REPORT.md) | 特性开关修复报告 | `FEATURE_FLAG_FIXES_REPORT.md` |

---

## 产品文档

### 用户文档

| 文档 | 描述 | 路径 |
|------|------|------|
| [README.md](../README.md) | 项目主README | `README.md` |
| [docs-product/README.md](../docs-product/README.md) | 产品文档首页 | `docs-product/README.md` |
| [docs-product/index.md](../docs-product/index.md) | 产品索引 | `docs-product/index.md` |
| [docs-product/SUMMARY.md](../docs-product/SUMMARY.md) | 功能摘要 | `docs-product/SUMMARY.md` |
| [docs-product/CONTRIBUTING.md](../docs-product/CONTRIBUTING.md) | 产品贡献指南 | `docs-product/CONTRIBUTING.md` |
| [docs-product/DEPLOY.md](../docs-product/DEPLOY.md) | 产品部署指南 | `docs-product/DEPLOY.md` |
| [docs-product/SCREENSHOTS.md](../docs-product/SCREENSHOTS.md) | 产品截图 | `docs-product/SCREENSHOTS.md` |

### 多语言文档

| 语言 | 路径 |
|------|------|
| 英文 | `docs-product/en/` |
| 中文 | `docs-product/zh/` |
| 日文 | `docs-product/ja/` |

---

## 其他文档

### 变更日志

| 文档 | 描述 | 路径 |
|------|------|------|
| [CHANGELOG.md](../CHANGELOG.md) | 版本变更日志 | `CHANGELOG.md` |

### 发布记录

| 文档 | 描述 | 路径 |
|------|------|------|
| [releases/PRO_V0.3.0_RELEASE_NOTES.md](../releases/PRO_V0.3.0_RELEASE_NOTES.md) | Pro v0.3.0发布说明 | `releases/PRO_V0.3.0_RELEASE_NOTES.md` |
| [releases/PRO_V0.3.0_VALIDATION_REPORT.md](../releases/PRO_V0.3.0_VALIDATION_REPORT.md) | Pro v0.3.0验证报告 | `releases/PRO_V0.3.0_VALIDATION_REPORT.md` |

### 项目日志

| 文档 | 描述 | 路径 |
|------|------|------|
| [borrow_checker_fix_log.md](../borrow_checker_fix_log.md) | Borrow checker修复日志 | `borrow_checker_fix_log.md` |
| [IMPLEMENTATION_SUMMARY.md](../IMPLEMENTATION_SUMMARY.md) | 实现总结 | `IMPLEMENTATION_SUMMARY.md` |
| [UX_IMPROVEMENT_REPORT.md](../UX_IMPROVEMENT_REPORT.md) | UX改进报告 | `UX_IMPROVEMENT_REPORT.md` |

### 工具文档

| 文档 | 描述 | 路径 |
|------|------|------|
| [BABYSITTER.md](../BABYSITTER.md) | Babysitter监控工具 | `BABYSITTER.md` |
| [api-tester/README.md](../api-tester/README.md) | API测试工具 | `api-tester/README.md` |
| [pro-server/README.md](../pro-server/README.md) | Pro服务器文档 | `pro-server/README.md` |
| [pro-server/INCIDENT_RESPONSE_README.md](../pro-server/INCIDENT_RESPONSE_README.md) | 事件响应文档 | `pro-server/INCIDENT_RESPONSE_README.md` |
| [installer/README.md](../installer/README.md) | 安装程序文档 | `installer/README.md` |
| [installer/CODE_SIGNING.md](../installer/CODE_SIGNING.md) | 代码签名文档 | `installer/CODE_SIGNING.md` |
| [installer/INSTALLER_SUMMARY.md](../installer/INSTALLER_SUMMARY.md) | 安装程序总结 | `installer/INSTALLER_SUMMARY.md` |
| [tests/README.md](../tests/README.md) | 测试目录文档 | `tests/README.md` |

### DOCS.RS准备

| 文档 | 描述 | 路径 |
|------|------|------|
| [DOCS_RS.md](../DOCS_RS.md) | docs.rs发布准备 | `DOCS_RS.md` |

---

## 文档统计

| 类别 | 数量 | 位置 |
|------|------|------|
| 产品规划 | 5 | `docs/` |
| 架构设计 | 7 | `docs/architecture/` |
| 开发者指南 | 7 | `docs/developers/` |
| 标准规范 | 4 | `docs/standards/` |
| 安全文档 | 4 | `docs/` (待归档) |
| 部署运维 | 9 | `docs/` |
| 分析报告 | 11 | `docs/` |
| 功能实现 | 12 | `docs/` |
| 产品文档 | 7 | `docs-product/` |
| 根级文档 | 35+ | 项目根目录 |
| **总计** | **101+** | - |

---

## 文档维护

### 新增文档流程

1. 根据文档类型选择正确目录
2. 更新本索引文件 (INDEX.md)
3. 在CLAUDE.md中更新文档结构说明
4. 确保文档格式符合[标准规范](standards/code-quality.md)

### 文档命名规范

- 使用大驼峰命名法（如 `FeatureImplementation.md`）
- 或使用短横线命名法（如 `feature-implementation.md`）
- 报告类文档使用 `_REPORT.md` 后缀
- 实现文档使用 `_IMPLEMENTATION.md` 后缀

---

*本文档由 EasySSH 文档归档系统生成 - 2026-04-01*
