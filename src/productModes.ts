export type ProductMode = 'lite' | 'standard' | 'pro';

export interface ProductModeMeta {
  id: ProductMode;
  title: string;
  subtitle: string;
  description: string;
  surface: string;
  capabilities: string[];
}

export const PRODUCT_MODES: ProductModeMeta[] = [
  {
    id: 'lite',
    title: 'EasySSH Lite',
    subtitle: 'SSH 配置保险箱',
    description: '专注配置、分组和快速连接，不把终端渲染塞进 Lite。',
    surface: 'Vault',
    capabilities: ['安全存储', '快速连接', '本地搜索'],
  },
  {
    id: 'standard',
    title: 'EasySSH Standard',
    subtitle: '终端工作台',
    description: '用于多会话、分屏和嵌入式终端的主工作区。',
    surface: 'Workspace',
    capabilities: ['嵌入式终端', '分屏布局', '多会话切换'],
  },
  {
    id: 'pro',
    title: 'EasySSH Pro',
    subtitle: '团队控制台',
    description: '用于团队协作、审计、SSO 和共享资源治理。',
    surface: 'Governance',
    capabilities: ['RBAC', '审计日志', 'SSO'],
  },
];

const PRODUCT_MODE_LOOKUP = Object.fromEntries(
  PRODUCT_MODES.map((mode) => [mode.id, mode])
) as Record<ProductMode, ProductModeMeta>;

export const getProductModeMeta = (mode: ProductMode) => PRODUCT_MODE_LOOKUP[mode];
