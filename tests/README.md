# EasySSH Testing Guide

## Overview

This document describes the comprehensive testing infrastructure for EasySSH, ensuring Apple-level quality through visual regression, accessibility, and performance testing.

## Testing Stack

| Tool | Purpose | Coverage |
|------|---------|----------|
| **Playwright** | E2E, Visual Regression | UI components, flows |
| **Vitest** | Unit Testing | React components, utilities |
| **@axe-core/playwright** | Accessibility | WCAG 2.1 AA compliance |
| **Lighthouse CI** | Performance | Core Web Vitals |
| **Storybook** | Component Isolation | Visual testing baseline |
| **cargo test** | Rust Unit Tests | Core library |

## Directory Structure

```
tests/
├── e2e/                    # End-to-end tests
│   ├── critical-flows.spec.ts    # Critical user flows
│   └── performance.spec.ts       # Performance benchmarks
├── visual/                 # Visual regression tests
│   ├── appshell.spec.ts          # AppShell component
│   └── sidebar.spec.ts           # Sidebar component
├── a11y/                   # Accessibility tests
│   └── accessibility.spec.ts     # axe-core scans
├── fixtures/               # Test data and files
├── utils/                  # Testing utilities
│   └── helpers.ts               # Common test functions
├── package.json            # Test dependencies
└── tsconfig.json           # TypeScript config

.github/workflows/
└── test.yml                # CI/CD test pipeline

playwright.config.ts        # Playwright configuration
```

## Quick Start

### Installation

```bash
# Install Playwright dependencies
cd tests
npm install

# Install browsers
npx playwright install

# For Windows WebView2 testing (Windows only)
npx playwright install chromium
```

### Running Tests

```bash
cd tests

# Run all tests
npm test

# Run with UI mode (for debugging)
npm run test:ui

# Run specific test file
npx playwright test visual/appshell.spec.ts

# Run in headed mode (see browser)
npm run test:headed

# Update visual snapshots
npm run update-snapshots
```

### Test Projects

```bash
# Visual regression only
npm run test:visual

# Accessibility only
npm run test:a11y

# Specific browser
npm run test:chromium
npm run test:firefox
npm run test:webkit
```

## Visual Regression Testing

### Baseline Screenshots

Baseline screenshots are stored in:
```
tests/visual/__snapshots__/
  └── {projectName}/
      └── {testFilePath}/
          └── {snapshotName}.png
```

### Creating New Visual Tests

```typescript
import { test, expect } from '@playwright/test';

test('component renders correctly', async ({ page }) => {
  await page.goto('/');
  await page.waitForSelector('[data-testid="my-component"]');

  // Take screenshot
  await expect(page).toHaveScreenshot('my-component.png', {
    fullPage: true,
  });
});
```

### Visual Diff Configuration

Playwright allows:
- **Threshold**: 0.2% pixel difference allowed
- **Max diff pixels**: 100 pixels
- **Animations**: Disabled for consistent screenshots
- **Viewport**: 1280x720 (configurable)
- **Device scale factor**: 2x for Retina quality

### Updating Snapshots

When intentional UI changes occur:

```bash
npm run update-snapshots
```

Review diffs carefully before committing new baselines.

## Accessibility Testing

### WCAG 2.1 AA Compliance

All tests use `@axe-core/playwright` to check:

- **Keyboard navigation**: All interactive elements reachable
- **ARIA labels**: Proper labeling for screen readers
- **Color contrast**: 4.5:1 for normal text, 3:1 for large text
- **Heading order**: Logical heading hierarchy
- **Form labels**: All inputs properly labeled

### Running Accessibility Tests

```bash
npm run test:a11y
```

### Accessibility Test Example

```typescript
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('page has no accessibility violations', async ({ page }) => {
  await page.goto('/');

  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
    .analyze();

  expect(results.violations).toEqual([]);
});
```

### Accessibility Requirements

| Component | Requirement |
|-----------|-------------|
| Server items | `aria-label` with server name and status |
| Connection status | `aria-live="polite"` for status changes |
| Terminal | Keyboard shortcuts documented |
| Buttons | Visible focus indicators |
| Forms | Error messages linked to inputs |

## Critical Test Scenarios

### 1. Connection Flow

```typescript
test('user can add server and connect', async ({ page }) => {
  // Add server
  await page.click('[data-testid="add-server-button"]');
  await page.fill('[data-testid="server-name-input"]', 'Test Server');
  await page.fill('[data-testid="server-host-input"]', 'test.example.com');
  await page.click('[data-testid="save-server-button"]');

  // Connect
  await page.click('text=Test Server');
  await page.click('[data-testid="connect-button"]');

  // Verify terminal opens
  await expect(page.locator('[data-testid="terminal"]')).toBeVisible();
});
```

### 2. Terminal Operations

- Command input and output
- Special key handling (Ctrl+C, Tab)
- Resize operations
- Copy/paste functionality
- Theme consistency

### 3. Monitor Panel

- CPU/Memory/Disk charts render
- Data refresh (auto and manual)
- Panel toggle states

### 4. SFTP File Management

- Directory navigation
- File upload
- File download
- Directory creation

### 5. Theming

- Dark/light mode toggle
- Preference persistence
- Terminal theme sync

## Performance Testing

### Terminal FPS Benchmarks

```typescript
test('terminal maintains 60fps during output', async ({ page }) => {
  // Generate high-volume output
  await page.evaluate(() => {
    const terminal = (window as any).terminal;
    for (let i = 0; i < 1000; i++) {
      terminal.writeln(`Line ${i}: ...`);
    }
  });

  // Measure frame rate
  const fps = await measureFrameRate(page, 2000);
  expect(fps).toBeGreaterThan(30);
});
```

### Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Terminal FPS | 60fps | >30fps |
| Initial render | <2s | <5s |
| Server list (1000 items) | <1s | <3s |
| Search response | <100ms | <500ms |
| Theme switch | <100ms | <300ms |
| Memory growth | <20MB | <50MB |

### Memory Leak Detection

```typescript
test('no memory leaks when opening/closing terminals', async ({ page }) => {
  const snapshots: number[] = [];

  // Open/close terminal 10 times
  for (let i = 0; i < 10; i++) {
    await openTerminal(page);
    await closeTerminal(page);
    snapshots.push(await getMemoryUsage(page));
  }

  // Check for unbounded growth
  const growth = calculateGrowth(snapshots);
  expect(growth).toBeLessThan(20 * 1024 * 1024); // 20MB
});
```

## CI/CD Integration

### GitHub Actions Workflow

Tests run automatically on:
- Push to `main` or `develop`
- Pull requests to `main`

### Test Matrix

| Job | Platforms | Browsers |
|-----|-----------|----------|
| Rust Tests | Ubuntu, Windows, macOS | - |
| Playwright | Ubuntu, Windows | Chromium, Firefox, WebKit |
| Visual Regression | Windows | Chromium |
| Accessibility | Ubuntu | Chromium |
| Performance | Windows | Chromium |

### Artifact Retention

- Test results: 30 days
- Screenshots on failure: 7 days
- Visual diffs: 30 days

## Test Data Attributes

Components should include `data-testid` attributes:

```html
<!-- AppShell -->
<div data-testid="app-shell">
  <header data-testid="app-header">...</header>
  <aside data-testid="sidebar">...</aside>
  <main data-testid="main-content">...</main>
</div>

<!-- Server Item -->
<div data-testid="server-item" data-server-id="123">
  <span data-testid="connection-status" data-status="connected"></span>
  <span data-testid="server-name">Production</span>
</div>

<!-- Terminal -->
<div data-testid="terminal">
  <div class="xterm-screen">...</div>
</div>
```

## Utility Functions

### Mock Helpers

```typescript
import { mockServers, mockConnection, openTerminal } from './utils/helpers';

// Mock server data
await mockServers(page, [
  { id: '1', name: 'Server 1', host: 'host1.com', status: 'connected' },
  { id: '2', name: 'Server 2', host: 'host2.com', status: 'disconnected' },
]);

// Mock connection
await mockConnection(page, {
  id: '1',
  name: 'Server 1',
  status: 'connected',
});

// Open terminal
await openTerminal(page, 'server-1');
```

### Theme Helpers

```typescript
import { setTheme, toggleTheme } from './utils/helpers';

// Set specific theme
await setTheme(page, 'dark');

// Toggle theme
const newTheme = await toggleTheme(page);
expect(['dark', 'light']).toContain(newTheme);
```

## Debugging Tests

### Debug Mode

```bash
# Step through test execution
npx playwright test --debug

# Slow motion execution
npx playwright test --headed --slowmo 1000

# Specific test with UI
npx playwright test visual/appshell.spec.ts --ui
```

### Viewing Reports

```bash
# Open HTML report
npm run report

# View in browser
cd tests && npx playwright show-report
```

### Common Issues

| Issue | Solution |
|-------|----------|
| Flaky visual tests | Add `await page.waitForTimeout(300)` before screenshot |
| Timeout on CI | Increase timeout in `playwright.config.ts` |
| Snapshot mismatch | Run `npm run update-snapshots` locally and review |
| WebView2 not found | Install Edge Dev channel or use `--channel=dev` |

## Adding New Tests

### 1. Component Visual Test

```typescript
// tests/visual/new-component.spec.ts
import { test, expect } from '@playwright/test';

test.describe('NewComponent Visual', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="new-component"]');
  });

  test('renders correctly', async ({ page }) => {
    const component = page.locator('[data-testid="new-component"]');
    await expect(component).toHaveScreenshot('new-component.png');
  });
});
```

### 2. E2E Flow Test

```typescript
// tests/e2e/my-feature.spec.ts
import { test, expect } from '@playwright/test';
import { waitForAppLoad, mockServers } from '../utils/helpers';

test.describe('My Feature', () => {
  test('complete user flow', async ({ page }) => {
    await page.goto('/');
    await waitForAppLoad(page);

    // Test steps...
  });
});
```

### 3. Accessibility Test

```typescript
// tests/a11y/my-component.spec.ts
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('no a11y violations', async ({ page }) => {
  await page.goto('/');

  const results = await new AxeBuilder({ page })
    .include('[data-testid="my-component"]')
    .analyze();

  expect(results.violations).toEqual([]);
});
```

## Best Practices

1. **Use data-testid**: Prefer `data-testid` over CSS selectors
2. **Mock data**: Use helper functions for consistent test data
3. **Wait properly**: Use explicit waits rather than arbitrary timeouts
4. **Isolate tests**: Each test should be independent
5. **Clean up**: Reset state between tests
6. **Document**: Add comments explaining complex test flows
7. **Review diffs**: Always review visual diffs before updating baselines

## Coverage Requirements

| Category | Target |
|----------|--------|
| Component unit tests | 100% |
| Visual snapshots | All UI states |
| Accessibility | WCAG 2.1 AA |
| Terminal FPS | 60fps |
| Memory leaks | None detected |

## Resources

- [Playwright Documentation](https://playwright.dev/)
- [axe-core Rules](https://dequeuniversity.com/rules/axe/4.9)
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [Vitest Documentation](https://vitest.dev/)
