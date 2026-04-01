import { test, expect } from '@playwright/test';

/**
 * Sidebar Visual Regression Tests
 *
 * Tests the sidebar component including:
 * - Server list items
 * - Group/folder structure
 * - Connection status indicators
 * - Hover and active states
 * - Context menus
 */

test.describe('Sidebar Visual Regression', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="sidebar"]', { timeout: 10000 });
  });

  test('Sidebar renders with server list', async ({ page }) => {
    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toBeVisible();

    // Populate with test data
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          servers: [
            { id: '1', name: 'Production Web', host: 'web.prod.example.com', status: 'connected' },
            { id: '2', name: 'Staging API', host: 'api.staging.example.com', status: 'disconnected' },
            { id: '3', name: 'Dev Database', host: 'db.dev.example.com', status: 'disconnected' },
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    await expect(sidebar).toHaveScreenshot('sidebar-server-list.png');
  });

  test('Sidebar with grouped servers', async ({ page }) => {
    const sidebar = page.locator('[data-testid="sidebar"]');

    // Load grouped servers
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          groups: [
            {
              name: 'Production',
              servers: [
                { id: 'p1', name: 'Web Server', status: 'connected' },
                { id: 'p2', name: 'Database', status: 'connected' },
              ]
            },
            {
              name: 'Staging',
              servers: [
                { id: 's1', name: 'Web Server', status: 'disconnected' },
              ]
            },
            {
              name: 'Development',
              servers: [
                { id: 'd1', name: 'Local VM', status: 'disconnected' },
              ]
            }
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    await expect(sidebar).toHaveScreenshot('sidebar-grouped.png');
  });

  test('Sidebar with expanded group', async ({ page }) => {
    // First load grouped servers
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          groups: [
            {
              name: 'Production',
              expanded: true,
              servers: [
                { id: 'p1', name: 'Web Server', status: 'connected' },
                { id: 'p2', name: 'Database', status: 'connected' },
              ]
            },
            {
              name: 'Staging',
              expanded: false,
              servers: [
                { id: 's1', name: 'Web Server', status: 'disconnected' },
              ]
            }
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toHaveScreenshot('sidebar-expanded-group.png');
  });

  test('Sidebar server item hover state', async ({ page }) => {
    // Load servers first
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          servers: [
            { id: '1', name: 'Test Server', host: 'test.example.com', status: 'disconnected' },
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    // Hover over server item
    const serverItem = page.locator('[data-testid="server-item"]').first();
    await serverItem.hover();
    await page.waitForTimeout(300);

    await expect(serverItem).toHaveScreenshot('sidebar-item-hover.png');
  });

  test('Sidebar server item active state', async ({ page }) => {
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          servers: [
            { id: '1', name: 'Test Server', host: 'test.example.com', status: 'connected', active: true },
            { id: '2', name: 'Other Server', host: 'other.example.com', status: 'disconnected' },
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toHaveScreenshot('sidebar-active-item.png');
  });

  test('Sidebar with connection status indicators', async ({ page }) => {
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          servers: [
            { id: '1', name: 'Connected Server', status: 'connected' },
            { id: '2', name: 'Disconnected Server', status: 'disconnected' },
            { id: '3', name: 'Connecting...', status: 'connecting' },
            { id: '4', name: 'Error State', status: 'error' },
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toHaveScreenshot('sidebar-status-indicators.png');
  });

  test('Sidebar search filter state', async ({ page }) => {
    // Open search
    await page.click('[data-testid="sidebar-search-button"]').catch(() => {});

    // Type search query
    const searchInput = page.locator('[data-testid="sidebar-search-input"]');
    if (await searchInput.isVisible().catch(() => false)) {
      await searchInput.fill('production');
      await page.waitForTimeout(300);

      await expect(page.locator('[data-testid="sidebar"]')).toHaveScreenshot('sidebar-search.png');
    }
  });

  test('Sidebar empty state', async ({ page }) => {
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: { servers: [] }
      }));
    });

    await page.waitForTimeout(500);

    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toHaveScreenshot('sidebar-empty.png');
  });

  test('Sidebar context menu', async ({ page }) => {
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          servers: [
            { id: '1', name: 'Test Server', status: 'connected' },
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    // Right-click on server item
    const serverItem = page.locator('[data-testid="server-item"]').first();
    await serverItem.click({ button: 'right' });
    await page.waitForTimeout(300);

    // Screenshot should include context menu
    await expect(page).toHaveScreenshot('sidebar-context-menu.png');
  });
});

test.describe('Sidebar Dark Mode', () => {
  test('Sidebar in dark mode', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="sidebar"]', { timeout: 10000 });

    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'dark');
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: {
          servers: [
            { id: '1', name: 'Production', status: 'connected' },
            { id: '2', name: 'Staging', status: 'disconnected' },
          ]
        }
      }));
    });

    await page.waitForTimeout(500);

    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toHaveScreenshot('sidebar-dark.png');
  });
});
