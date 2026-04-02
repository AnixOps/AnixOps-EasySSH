# EasySSH 版本发布流程

## 版本命名规范

### 版本号格式
`MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`

示例:
- 0.3.0-beta.1
- 0.4.0-rc.2
- 0.4.0
- 0.5.0-alpha.1

### 预发布标识
- `alpha`: 内部测试
- `beta`: 公开测试
- `rc`: 候选版本

## 发布流程

### 1. 开发阶段 (Development)
- 功能开发
- 代码审查
- 单元测试

### 2. 冻结阶段 (Feature Freeze)
- 停止新功能开发
- 专注于bug修复
- 文档完善

### 3. 测试阶段 (Testing)
- 集成测试
- 性能测试
- 安全审计

### 4. 候选阶段 (RC)
- 发布RC版本
- 社区测试
- Bug修复

### 5. 发布阶段 (Release)
- 创建GitHub Release
- 构建二进制包
- 更新文档

### 6. 发布后 (Post-Release)
- 监控反馈
- 紧急修复
- 计划下个版本

## 三版本同步发布

所有版本(Lite/Standard/Pro)同步发布:
```
v0.3.0-beta
├── easyssh-lite-v0.3.0-beta-windows-x64.exe
├── easyssh-lite-v0.3.0-beta-linux-x64.AppImage
├── easyssh-standard-v0.3.0-beta-windows-x64.exe
├── easyssh-standard-v0.3.0-beta-linux-x64.AppImage
├── easyssh-pro-v0.3.0-beta-windows-x64.exe
└── easyssh-pro-server-v0.3.0-beta-docker.tar.gz
```

## 质量门禁

### Beta发布
- [ ] 编译零错误
- [ ] 测试覆盖率>75%
- [ ] 安全扫描通过
- [ ] 文档完整

### Stable发布
- [ ] Beta反馈处理完成
- [ ] 测试覆盖率>85%
- [ ] 性能基准达标
- [ ] 企业客户验证

### Enterprise发布
- [ ] 合规认证通过
- [ ] 压力测试通过
- [ ] 安全渗透测试
- [ ] 企业客户试点

## 回滚策略

### 发现严重bug
1. 立即停止下载
2. 发布修复版本 (PATCH版本)
3. 通知用户更新

### 版本降级
```bash
# 用户降级命令
easyssh --version-check
easyssh --update-to=0.3.0-beta.1
```
