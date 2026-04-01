# EasySSH 产品文档站点

## 构建指南

### 开发环境要求

- Node.js 18+
- pnpm 8+
- Git

### 安装

```bash
cd docs-product
pnpm install
```

### 开发模式

```bash
pnpm docs:dev
```

访问 http://localhost:5173

### 构建

```bash
pnpm docs:build
```

输出目录: `.vitepress/dist`

### 预览构建结果

```bash
pnpm docs:preview
```

## 部署

### GitHub Pages

1. 推送代码到 GitHub
2. 在仓库设置中启用 GitHub Pages
3. 选择 Source: GitHub Actions
4. 自动部署工作流已配置

### 自定义域名

在 `public/` 目录创建 `CNAME` 文件:
```
docs.easyssh.dev
```

### 手动部署

```bash
# 构建
pnpm docs:build

# 部署到服务器
rsync -avz .vitepress/dist/ user@server:/var/www/docs/
```

## 多语言部署

站点会自动构建多语言版本:
- 中文: `/zh/`
- 英文: `/en/`
- 日文: `/ja/`

根路径 `/` 默认重定向到中文版本。

## 搜索配置

文档使用 Algolia DocSearch。申请配置:
1. 访问 https://docsearch.algolia.com/apply/
2. 提交网站信息
3. 更新 `.vitepress/config.ts` 中的搜索配置

## 维护

### 更新依赖

```bash
pnpm update
```

### 检查链接

```bash
pnpm exec vitepress check .vitepress/dist
```

## 支持

- docs@easyssh.dev
- GitHub Issues
