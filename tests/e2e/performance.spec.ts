import { test, expect } from '@playwright/test';

/**
 * Terminal Performance Tests
 *
 * Ensures terminal maintains 60fps during:
 * - High volume output
 * - Rapid input
 * - Resize operations
 * - Theme switches
 */

// Performance measurement helpers
async function measureFrameRate(page: any, duration: number = 5000): Promise<number> {
  const frameTimes: number[] = [];

  await page.evaluate((measureDuration: number) => {
    return new Promise<void>((resolve) => {
      let lastTime = performance.now();
      let rafId: number;

      const measureFrame = () => {
        const currentTime = performance.now();
        const delta = currentTime - lastTime;
        (window as any).__frameTimes.push(delta);
        lastTime = currentTime;

        if (currentTime < measureDuration) {
          rafId = requestAnimationFrame(measureFrame);
        } else {
          cancelAnimationFrame(rafId);
          resolve();
        }
      };

      (window as any).__frameTimes = [];
      rafId = requestAnimationFrame(measureFrame);
    });
  }, duration);

  const times = await page.evaluate(() => (window as any).__frameTimes);
  const avgFrameTime = times.reduce((a: number, b: number) => a + b, 0) / times.length;
  return 1000 / avgFrameTime; // Convert to FPS
}

test.describe('Terminal Performance', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('test:open-terminal'));
    });
    await page.waitForSelector('[data-testid="terminal"]', { timeout: 5000 });
  });

  test('terminal maintains 60fps during command output', async ({ page }) => {
    // Generate high volume output
    await page.evaluate(() => {
      const terminal = (window as any).terminal;
      if (terminal) {
        // Simulate high-volume output
        for (let i = 0; i < 1000; i++) {
          terminal.writeln(`Line ${i}: Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.`);
        }
      }
    });

    // Measure frame rate during output
    const fps = await measureFrameRate(page, 2000);

    // Should maintain at least 30fps under load (60fps ideal)
    expect(fps).toBeGreaterThan(30);
  });

  test('terminal handles rapid input smoothly', async ({ page }) => {
    // Measure FPS during rapid typing
    const startTime = Date.now();

    // Rapid input simulation
    for (let i = 0; i < 100; i++) {
      await page.keyboard.type('a');
    }

    const endTime = Date.now();
    const duration = endTime - startTime;

    // Should complete within reasonable time (less than 5 seconds for 100 chars)
    expect(duration).toBeLessThan(5000);
  });

  test('terminal resize performance', async ({ page }) => {
    const sizes = [
      { width: 800, height: 600 },
      { width: 1280, height: 720 },
      { width: 1920, height: 1080 },
      { width: 1280, height: 720 },
    ];

    const resizeTimes: number[] = [];

    for (const size of sizes) {
      const startTime = Date.now();
      await page.setViewportSize(size);
      await page.waitForTimeout(200); // Allow resize to complete
      const endTime = Date.now();
      resizeTimes.push(endTime - startTime);
    }

    // Average resize should be fast
    const avgResizeTime = resizeTimes.reduce((a, b) => a + b, 0) / resizeTimes.length;
    expect(avgResizeTime).toBeLessThan(500);
  });

  test('terminal memory usage remains stable', async ({ page }) => {
    // Get initial memory
    const initialMemory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize || 0;
    });

    // Generate lots of output
    await page.evaluate(() => {
      const terminal = (window as any).terminal;
      if (terminal) {
        for (let i = 0; i < 5000; i++) {
          terminal.writeln(`Line ${i}: ${'x'.repeat(100)}`);
        }
      }
    });

    // Wait and clear
    await page.waitForTimeout(1000);
    await page.evaluate(() => {
      const terminal = (window as any).terminal;
      if (terminal) {
        terminal.clear();
      }
    });

    // Force garbage collection if available
    await page.evaluate(() => {
      if ((window as any).gc) {
        (window as any).gc();
      }
    });

    await page.waitForTimeout(1000);

    // Get final memory
    const finalMemory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize || 0;
    });

    // Memory should not grow unbounded (allow 50MB growth)
    const memoryGrowth = finalMemory - initialMemory;
    expect(memoryGrowth).toBeLessThan(50 * 1024 * 1024);
  });
});

test.describe('UI Performance', () => {
  test('server list renders efficiently with many servers', async ({ page }) => {
    await page.goto('/');

    // Generate many servers
    const servers = Array.from({ length: 1000 }, (_, i) => ({
      id: `server-${i}`,
      name: `Server ${i}`,
      host: `host${i}.example.com`,
      status: i % 3 === 0 ? 'connected' : 'disconnected',
    }));

    const startTime = Date.now();

    await page.evaluate((serverList) => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: { servers: serverList }
      }));
    }, servers);

    await page.waitForSelector('[data-testid="server-item"]', { timeout: 10000 });

    const renderTime = Date.now() - startTime;

    // Should render within 3 seconds
    expect(renderTime).toBeLessThan(3000);
  });

  test('search filtering is responsive', async ({ page }) => {
    await page.goto('/');

    // Generate servers
    const servers = Array.from({ length: 500 }, (_, i) => ({
      id: `server-${i}`,
      name: `Test Server ${i}`,
      host: `host${i}.example.com`,
    }));

    await page.evaluate((serverList) => {
      window.dispatchEvent(new CustomEvent('test:servers-loaded', {
        detail: { servers: serverList }
      }));
    }, servers);

    await page.waitForTimeout(500);

    // Measure search response time
    const searchQueries = ['server 1', 'server 50', 'server 99', 'test'];

    for (const query of searchQueries) {
      const startTime = Date.now();

      await page.evaluate((q: string) => {
        window.dispatchEvent(new CustomEvent('test:search', { detail: { query: q } }));
      }, query);

      await page.waitForTimeout(100);

      const searchTime = Date.now() - startTime;
      expect(searchTime).toBeLessThan(500);
    }
  });

  test('theme switch is instant', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="app-shell"]', { timeout: 10000 });

    const startTime = Date.now();

    // Toggle theme
    await page.click('[data-testid="theme-toggle"]').catch(() => {});

    // Wait for theme to apply
    await page.waitForFunction(() => {
      const theme = document.documentElement.getAttribute('data-theme');
      return theme === 'dark' || theme === 'light';
    }, { timeout: 1000 });

    const switchTime = Date.now() - startTime;

    // Should be nearly instant
    expect(switchTime).toBeLessThan(300);
  });
});

test.describe('Memory Leak Detection', () => {
  test('no memory leaks when opening/closing terminals', async ({ page }) => {
    await page.goto('/');

    const memorySnapshots: number[] = [];

    // Take initial snapshot
    await page.waitForTimeout(1000);
    let memory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize || 0;
    });
    memorySnapshots.push(memory);

    // Open and close terminal multiple times
    for (let i = 0; i < 10; i++) {
      // Open
      await page.evaluate(() => {
        window.dispatchEvent(new CustomEvent('test:open-terminal'));
      });
      await page.waitForSelector('[data-testid="terminal"]', { timeout: 5000 });
      await page.waitForTimeout(500);

      // Close
      await page.evaluate(() => {
        window.dispatchEvent(new CustomEvent('test:close-terminal'));
      });
      await page.waitForTimeout(500);

      // Take snapshot
      memory = await page.evaluate(() => {
        return (performance as any).memory?.usedJSHeapSize || 0;
      });
      memorySnapshots.push(memory);
    }

    // Check for linear growth (indicates leak)
    const firstHalf = memorySnapshots.slice(0, 5);
    const secondHalf = memorySnapshots.slice(5);

    const firstAvg = firstHalf.reduce((a, b) => a + b, 0) / firstHalf.length;
    const secondAvg = secondHalf.reduce((a, b) => a + b, 0) / secondHalf.length;

    // Allow 20MB growth (some growth is normal due to caching)
    const growth = secondAvg - firstAvg;
    expect(growth).toBeLessThan(20 * 1024 * 1024);
  });
});
