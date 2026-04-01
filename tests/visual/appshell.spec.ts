import { test, expect } from '@playwright/test';

/**
 * AppShell Visual Regression Tests
 *
 * Tests the main application shell layout including:
 * - Header with title and controls
 * - Navigation sidebar
 * - Main content area
 * - Overall layout proportions
 */

test.describe('AppShell Visual Regression', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/');
    // Wait for the app to be fully loaded
    await page.waitForSelector('[data-testid="app-shell"]', { timeout: 10000 });
  });

  test('AppShell renders correctly in default state', async ({ page }) => {
    const appShell = page.locator('[data-testid="app-shell"]');
    await expect(appShell).toBeVisible();

    // Take full page screenshot
    await expect(page).toHaveScreenshot('appshell-default.png', {
      fullPage: true,
    });
  });

  test('AppShell with sidebar expanded', async ({ page }) => {
    // Ensure sidebar is expanded (if it has a toggle)
    const sidebar = page.locator('[data-testid="sidebar"]');
    const isExpanded = await sidebar.getAttribute('data-expanded');

    if (isExpanded === 'false') {
      // Click expand button if exists
      await page.click('[data-testid="sidebar-toggle"]').catch(() => {});
      await page.waitForTimeout(300); // Wait for animation
    }

    await expect(sidebar).toHaveScreenshot('appshell-sidebar-expanded.png');
  });

  test('AppShell with sidebar collapsed', async ({ page }) => {
    // Collapse sidebar if it has a toggle
    const sidebar = page.locator('[data-testid="sidebar"]');
    const isExpanded = await sidebar.getAttribute('data-expanded');

    if (isExpanded !== 'false') {
      await page.click('[data-testid="sidebar-toggle"]').catch(() => {});
      await page.waitForTimeout(300);
    }

    await expect(sidebar).toHaveScreenshot('appshell-sidebar-collapsed.png');
  });

  test('AppShell header is consistent', async ({ page }) => {
    const header = page.locator('[data-testid="app-header"]');
    await expect(header).toBeVisible();
    await expect(header).toHaveScreenshot('appshell-header.png');
  });

  test('AppShell with active connection', async ({ page }) => {
    // Mock an active connection state
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:connection-active', {
        detail: { id: 'test-123', name: 'Test Server' }
      }));
    });

    await page.waitForTimeout(500);

    await expect(page).toHaveScreenshot('appshell-active-connection.png', {
      fullPage: true,
    });
  });

  test('AppShell responsive behavior - tablet', async ({ page }) => {
    // Test at tablet viewport
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.waitForTimeout(300);

    await expect(page).toHaveScreenshot('appshell-tablet.png', {
      fullPage: true,
    });
  });

  test('AppShell responsive behavior - mobile', async ({ page }) => {
    // Test at mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.waitForTimeout(300);

    await expect(page).toHaveScreenshot('appshell-mobile.png', {
      fullPage: true,
    });
  });
});

test.describe('AppShell Dark Mode', () => {
  test('AppShell renders correctly in dark mode', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="app-shell"]', { timeout: 10000 });

    // Force dark mode
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'dark');
    });
    await page.waitForTimeout(300);

    await expect(page).toHaveScreenshot('appshell-dark.png', {
      fullPage: true,
    });
  });
});

test.describe('AppShell Light Mode', () => {
  test('AppShell renders correctly in light mode', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="app-shell"]', { timeout: 10000 });

    // Force light mode
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'light');
    });
    await page.waitForTimeout(300);

    await expect(page).toHaveScreenshot('appshell-light.png', {
      fullPage: true,
    });
  });
});
