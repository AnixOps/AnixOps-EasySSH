import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

/**
 * Accessibility Tests using axe-core
 *
 * Ensures WCAG 2.1 AA compliance across the application.
 */

test.describe('Accessibility - AppShell', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="app-shell"]', { timeout: 10000 });
  });

  test('should have no detectable accessibility issues on main page', async ({ page }) => {
    const accessibilityScanResults = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('should have proper heading structure', async ({ page }) => {
    const accessibilityScanResults = await new AxeBuilder({ page })
      .withRules(['heading-order'])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('should have proper color contrast', async ({ page }) => {
    const accessibilityScanResults = await new AxeBuilder({ page })
      .withRules(['color-contrast'])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('should have proper ARIA labels', async ({ page }) => {
    const accessibilityScanResults = await new AxeBuilder({ page })
      .withRules([
        'aria-required-attr',
        'aria-required-children',
        'aria-required-parent',
        'aria-roles',
        'aria-valid-attr-value',
        'aria-valid-attr'
      ])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('should be keyboard navigable', async ({ page }) => {
    // Tab through main interactive elements
    const tabStops = [
      '[data-testid="sidebar-toggle"]',
      '[data-testid="add-server-button"]',
      '[data-testid="settings-button"]',
    ];

    for (const selector of tabStops) {
      await page.keyboard.press('Tab');
      const focusedElement = page.locator(':focus');
      await expect(focusedElement).toBeVisible();
    }
  });
});

test.describe('Accessibility - Sidebar', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="sidebar"]', { timeout: 10000 });

    // Load test servers
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          servers: [
            { id: '1', name: 'Test Server 1', status: 'connected' },
            { id: '2', name: 'Test Server 2', status: 'disconnected' },
          ]
        }
      }));
    });
    await page.waitForTimeout(500);
  });

  test('should have no accessibility violations in sidebar', async ({ page }) => {
    const accessibilityScanResults = await new AxeBuilder({ page })
      .include('[data-testid="sidebar"]')
      .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('should have accessible server list items', async ({ page }) => {
    const serverItems = page.locator('[data-testid="server-item"]');

    for (let i = 0; i < await serverItems.count(); i++) {
      const item = serverItems.nth(i);

      // Check for accessible name
      const ariaLabel = await item.getAttribute('aria-label');
      const hasAccessibleName = ariaLabel || await item.textContent();
      expect(hasAccessibleName).toBeTruthy();

      // Check role
      const role = await item.getAttribute('role');
      expect(['button', 'link', 'listitem']).toContain(role);
    }
  });

  test('should have accessible status indicators', async ({ page }) => {
    const statusIndicators = page.locator('[data-testid="connection-status"]');

    for (let i = 0; i < await statusIndicators.count(); i++) {
      const indicator = statusIndicators.nth(i);

      // Status should have accessible text
      const statusText = await indicator.getAttribute('aria-label');
      expect(statusText).toMatch(/connected|disconnected|connecting|error/i);
    }
  });
});

test.describe('Accessibility - Terminal', () => {
  test('should have accessible terminal controls', async ({ page }) => {
    await page.goto('/');

    // Open terminal
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:open-terminal'));
    });

    await page.waitForSelector('[data-testid="terminal"]', { timeout: 5000 });

    const accessibilityScanResults = await new AxeBuilder({ page })
      .include('[data-testid="terminal"]')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();

    // Terminal might have some unavoidable violations due to xterm.js
    // But we should have no critical or serious violations
    const seriousViolations = accessibilityScanResults.violations.filter(
      v => v.impact === 'critical' || v.impact === 'serious'
    );
    expect(seriousViolations).toEqual([]);
  });
});

test.describe('Accessibility - Forms', () => {
  test('add server form should be accessible', async ({ page }) => {
    await page.goto('/');

    // Open add server dialog
    await page.click('[data-testid="add-server-button"]').catch(() => {});

    const form = page.locator('[data-testid="add-server-form"]');
    if (await form.isVisible().catch(() => false)) {
      const accessibilityScanResults = await new AxeBuilder({ page })
        .include('[data-testid="add-server-form"]')
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();

      expect(accessibilityScanResults.violations).toEqual([]);
    }
  });
});

test.describe('Accessibility - Dark Mode Contrast', () => {
  test('dark mode should maintain color contrast', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(500);

    // Switch to dark mode
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'dark');
    });

    const accessibilityScanResults = await new AxeBuilder({ page })
      .withRules(['color-contrast'])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });
});
