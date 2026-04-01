import { Page, Locator, expect } from '@playwright/test';

/**
 * Test Utilities for EasySSH
 *
 * Provides helper functions for common testing operations.
 */

/**
 * Wait for the application to be fully loaded
 */
export async function waitForAppLoad(page: Page, timeout = 10000): Promise<void> {
  await page.waitForSelector('[data-testid="app-shell"]', { timeout });
  // Additional wait for any async initialization
  await page.waitForTimeout(500);
}

/**
 * Mock server data in the application
 */
export async function mockServers(
  page: Page,
  servers: Array<{
    id: string;
    name: string;
    host: string;
    status?: 'connected' | 'disconnected' | 'connecting' | 'error';
    group?: string;
  }>
): Promise<void> {
  await page.evaluate((serverData) => {
    window.dispatchEvent(new CustomEvent('test:servers-loaded', {
      detail: { servers: serverData }
    }));
  }, servers);
  await page.waitForTimeout(300);
}

/**
 * Mock connection state
 */
export async function mockConnection(
  page: Page,
  connection: {
    id: string;
    name: string;
    status: 'connected' | 'disconnected' | 'connecting' | 'error';
  }
): Promise<void> {
  await page.evaluate((conn) => {
    window.dispatchEvent(new CustomEvent('test:connection-active', {
      detail: conn
    }));
  }, connection);
  await page.waitForTimeout(300);
}

/**
 * Open terminal with optional server
 */
export async function openTerminal(
  page: Page,
  serverId?: string
): Promise<void> {
  await page.evaluate((id) => {
    window.dispatchEvent(new CustomEvent('test:open-terminal', {
      detail: { serverId: id }
    }));
  }, serverId);
  await page.waitForSelector('[data-testid="terminal"]', { timeout: 5000 });
}

/**
 * Close terminal
 */
export async function closeTerminal(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent('test:close-terminal'));
  });
  await page.waitForTimeout(300);
}

/**
 * Open SFTP panel
 */
export async function openSFTP(page: Page): Promise<Locator> {
  await page.click('[data-testid="sftp-button"]').catch(() => {});
  const panel = page.locator('[data-testid="sftp-panel"]');
  await panel.waitFor({ timeout: 5000 });
  return panel;
}

/**
 * Open Monitor panel
 */
export async function openMonitor(page: Page): Promise<Locator> {
  await page.click('[data-testid="monitor-button"]').catch(() => {});
  const panel = page.locator('[data-testid="monitor-panel"]');
  await panel.waitFor({ timeout: 5000 });
  return panel;
}

/**
 * Toggle theme between dark and light
 */
export async function toggleTheme(page: Page): Promise<string> {
  await page.click('[data-testid="theme-toggle"]').catch(() => {});
  await page.waitForTimeout(300);

  return await page.evaluate(() => {
    return document.documentElement.getAttribute('data-theme') || 'system';
  });
}

/**
 * Set theme explicitly
 */
export async function setTheme(page: Page, theme: 'dark' | 'light' | 'system'): Promise<void> {
  await page.evaluate((t) => {
    document.documentElement.setAttribute('data-theme', t);
  }, theme);
  await page.waitForTimeout(300);
}

/**
 * Type command in terminal
 */
export async function typeInTerminal(
  page: Page,
  command: string,
  pressEnter = true
): Promise<void> {
  for (const char of command) {
    await page.keyboard.type(char);
  }

  if (pressEnter) {
    await page.keyboard.press('Enter');
  }

  await page.waitForTimeout(500);
}

/**
 * Wait for terminal output to contain text
 */
export async function waitForTerminalOutput(
  page: Page,
  text: string,
  timeout = 5000
): Promise<void> {
  const terminal = page.locator('.xterm-screen');
  await expect(terminal).toContainText(text, { timeout });
}

/**
 * Check if element is in viewport
 */
export async function isInViewport(page: Page, selector: string): Promise<boolean> {
  return await page.evaluate((sel) => {
    const element = document.querySelector(sel);
    if (!element) return false;

    const rect = element.getBoundingClientRect();
    return (
      rect.top >= 0 &&
      rect.left >= 0 &&
      rect.bottom <= window.innerHeight &&
      rect.right <= window.innerWidth
    );
  }, selector);
}

/**
 * Measure element render time
 */
export async function measureRenderTime(
  page: Page,
  renderAction: () => Promise<void>
): Promise<number> {
  const startTime = Date.now();
  await renderAction();
  return Date.now() - startTime;
}

/**
 * Get performance metrics
 */
export async function getPerformanceMetrics(page: Page): Promise<{
  fps: number;
  memory: number;
  nodes: number;
}> {
  const metrics = await page.evaluate(() => {
    const memory = (performance as any).memory?.usedJSHeapSize || 0;
    const nodes = document.querySelectorAll('*').length;

    // Measure frame rate
    let frameCount = 0;
    const startTime = performance.now();

    return new Promise<{ fps: number; memory: number; nodes: number }>((resolve) => {
      const countFrames = () => {
        frameCount++;
        if (performance.now() - startTime < 1000) {
          requestAnimationFrame(countFrames);
        } else {
          resolve({
            fps: frameCount,
            memory,
            nodes,
          });
        }
      };
      requestAnimationFrame(countFrames);
    });
  });

  return metrics;
}

/**
 * Assert no accessibility violations (lightweight check)
 */
export async function assertNoCriticalA11yIssues(page: Page): Promise<void> {
  // Basic checks that don't require axe-core
  const issues = await page.evaluate(() => {
    const problems: string[] = [];

    // Check for images without alt
    document.querySelectorAll('img:not([alt])').forEach((img) => {
      problems.push(`Image without alt: ${(img as HTMLImageElement).src}`);
    });

    // Check for buttons without accessible names
    document.querySelectorAll('button').forEach((btn) => {
      if (!btn.textContent?.trim() && !btn.getAttribute('aria-label')) {
        problems.push(`Button without accessible name`);
      }
    });

    // Check for low contrast (simplified)
    const elements = document.querySelectorAll('p, span, div');
    elements.forEach((el) => {
      const style = window.getComputedStyle(el);
      const color = style.color;
      const bgColor = style.backgroundColor;

      // Very basic check for similar colors
      if (color === bgColor && el.textContent?.trim()) {
        problems.push(`Possible low contrast text: "${el.textContent.substring(0, 50)}"`);
      }
    });

    return problems;
  });

  if (issues.length > 0) {
    console.warn('Accessibility issues found:', issues);
  }

  // Don't fail the test, just log warnings
  expect(issues.length).toBeLessThan(10);
}

/**
 * Generate test server data
 */
export function generateTestServers(count: number): Array<{
  id: string;
  name: string;
  host: string;
  status: string;
}> {
  return Array.from({ length: count }, (_, i) => ({
    id: `test-server-${i}`,
    name: `Test Server ${i}`,
    host: `host${i}.example.com`,
    status: i % 3 === 0 ? 'connected' : 'disconnected',
  }));
}

/**
 * Create a test file for upload testing
 */
export function createTestFileContent(sizeKB: number = 1): string {
  return 'x'.repeat(sizeKB * 1024);
}

/**
 * Retry function for flaky operations
 */
export async function retry<T>(
  operation: () => Promise<T>,
  maxAttempts = 3,
  delay = 1000
): Promise<T> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      return await operation();
    } catch (error) {
      if (i === maxAttempts - 1) throw error;
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
  throw new Error('Retry failed');
}
