# EasySSH UI Component Library

## 概览

完整的跨平台UI组件库，支持Windows、Linux和macOS。

## 结构

```
design-system/           # React/TypeScript组件 (API Tester)
├── src/
│   ├── components/     # React组件
│   │   ├── Button.tsx  # 按钮组件
│   │   ├── Card.tsx    # 卡片组件
│   │   ├── Badge.tsx   # 徽章组件
│   │   ├── Input.tsx   # 输入组件
│   │   ├── Toast.tsx   # 通知组件
│   │   ├── Skeleton.tsx # 骨架屏
│   │   └── Modal.tsx   # 模态框
│   ├── theme/          # 主题系统
│   ├── icons/          # 图标系统
│   ├── animations/     # 动画工具
│   └── utils/          # 工具函数
├── src/tokens/         # 设计令牌
└── tailwind.config.ts  # Tailwind配置

crates/easyssh-platforms/shared-ui/  # Rust共享组件
├── src/
│   ├── lib.rs          # 主库
│   ├── theme.rs        # 主题系统
│   ├── animations.rs   # 动画系统
│   ├── icons.rs        # 图标系统
│   ├── layout.rs       # 布局系统
│   ├── accessibility.rs # 可访问性
│   └── components.rs   # 组件定义
└── Cargo.toml
```

## 功能特性

### 1. 共享UI组件 (Rust)
- **Theme System**: 亮色/暗色/高对比度模式
- **Animation System**: 支持减少动画偏好
- **Icon System**: 跨平台图标
- **Layout System**: 响应式布局
- **Accessibility**: WCAG 2.1 AA兼容

### 2. React组件 (TypeScript)
- **Button**: 多变体按钮，支持加载状态
- **Card**: 卡片容器，支持网格和服务器卡片
- **Badge**: 状态徽章，支持点和计数
- **Input**: 表单输入，支持密码、验证
- **Toast**: 通知系统，支持hook
- **Skeleton**: 加载骨架屏
- **Modal**: 模态框，支持确认和警告

### 3. 主题系统
- CSS变量驱动的主题
- 自动系统主题检测
- 高对比度模式支持
- 减少动画偏好检测

### 4. 动画系统
- 淡入淡出
- 滑动动画
- 缩放动画
- 交错动画
- 弹簧动画
- 滚动显示

### 5. 图标系统
- 50+ Lucide图标
- 平台特定图标集
- 可访问性支持

## 使用方法

### React组件

```tsx
import { Button, Card, Badge, ThemeProvider } from '@easyssh/design-system';

function App() {
  return (
    <ThemeProvider>
      <Button variant="primary" icon="check">
        Connect
      </Button>

      <Card title="Server" icon="server">
        <p>Connection details...</p>
      </Card>

      <Badge variant="success" dot>Online</Badge>
    </ThemeProvider>
  );
}
```

### Rust组件

```rust
use easyssh_shared_ui::{UIManager, Theme, ColorScheme};

let ui = UIManager::new();
ui.theme().set_scheme(ColorScheme::Dark);
```

## 依赖项

### React
- React 18+
- Tailwind CSS 3.4+
- class-variance-authority
- clsx
- tailwind-merge

### Rust
- serde
- thiserror
- easyssh-core

## 测试

```bash
# React组件测试
cd design-system
npm run typecheck

# Rust组件测试
cargo test -p easyssh-shared-ui
```

## 贡献指南

1. 组件应支持亮色/暗色/高对比度模式
2. 动画必须尊重减少动画偏好
3. 所有组件必须满足WCAG 2.1 AA标准
4. 图标需要可访问性标签
5. 添加新组件时需要更新测试

## 版本历史

- 1.1.0: 添加完整React组件库
- 1.0.0: 初始设计令牌和基础组件
