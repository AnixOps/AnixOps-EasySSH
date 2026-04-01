# EasySSH 文档站点配置

## 结构规划

```
docs-product/
├── .vitepress/
│   ├── config.ts          # 主配置
│   ├── config/en.ts       # 英文配置
│   ├── config/zh.ts       # 中文配置
│   ├── config/ja.ts       # 日文配置
│   └── theme/
│       └── index.ts         # 自定义主题
├── public/
│   ├── images/
│   │   ├── screenshots/     # 各平台截图
│   │   └── gifs/            # 功能演示GIF
│   └── logos/
├── guide/
│   ├── index.md             # 快速开始
│   ├── installation.md      # 安装指南
│   ├── getting-started.md   # 快速开始
│   └── features/            # 功能详解
│       ├── lite.md
│       ├── standard.md
│       └── pro.md
├── api/
│   ├── index.md             # API概览
│   ├── core/                # Core库API
│   │   ├── ssh.md
│   │   ├── db.md
│   │   ├── crypto.md
│   │   └── sftp.md
│   └── ffi.md               # FFI接口
├── develop/
│   ├── index.md             # 开发入门
│   ├── architecture.md      # 架构说明
│   ├── contributing.md      # 贡献指南
│   ├── building.md          # 构建指南
│   └── testing.md           # 测试指南
├── deploy/
│   ├── index.md             # 部署概览
│   ├── enterprise.md        # 企业部署
│   ├── security.md          # 安全配置
│   └── scaling.md           # 扩展指南
├── video/
│   ├── index.md             # 视频概览
│   ├── quick-tour.md        # 快速导览脚本
│   ├── lite-demo.md         # Lite版演示
│   ├── standard-demo.md     # Standard版演示
│   └── pro-demo.md          # Pro版演示
├── faq/
│   ├── index.md             # 常见问题
│   ├── general.md           # 一般问题
│   ├── lite.md              # Lite版问题
│   ├── standard.md          # Standard版问题
│   └── pro.md               # Pro版问题
├── troubleshooting/
│   ├── index.md             # 故障排查概览
│   ├── error-codes.md       # 错误代码
│   ├── connection.md        # 连接问题
│   ├── authentication.md    # 认证问题
│   └── performance.md       # 性能问题
├── releases/
│   ├── index.md             # 版本历史
│   ├── changelog.md         # 更新日志
│   ├── migration.md         # 迁移指南
│   └── roadmap.md           # 产品路线图
└── index.md                 # 首页
```

## 技术选型

- **框架**: VitePress 1.0+
- **主题**: 默认主题 + 自定义CSS
- **搜索**: Algolia DocSearch
- **评论**: Giscus (GitHub Discussions)
- **分析**: Plausible / Google Analytics

## 多语言配置

| 语言 | 代码 | 路径 |
|------|------|------|
| 简体中文 | zh | /zh/ |
| 英文 | en | /en/ |
| 日文 | ja | /ja/ |

## 部署目标

- 主站: https://docs.easyssh.dev
- 预览: https://docs-staging.easyssh.dev
