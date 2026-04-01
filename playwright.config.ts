import { defineConfig, devices } from '@playwright/test';
import path from 'path';

/**
 * EasySSH Playwright Configuration
 *
 * This configuration supports:
 * - Visual regression testing for Windows WebView2 UI
 * - Debug WebSocket interface testing
 * - E2E testing for critical user flows
 * - Accessibility testing with axe-core
 */

// Base URL for tests (adjust based on test target)
const baseURL = process.env.TEST_URL || 'http://localhost:1420';

export default defineConfig({
  testDir: './tests',

  // Run tests in files in parallel
  fullyParallel: true,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // Retry on CI only
  retries: process.env.CI ? 2 : 0,

  // Opt out of parallel tests on CI for stability
  workers: process.env.CI ? 1 : undefined,

  // Reporter to use
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['json', { outputFile: 'playwright-report/test-results.json' }],
    process.env.CI ? ['github'] : ['line'],
  ],

  // Shared settings for all projects
  use: {
    // Base URL to use in actions like `await page.goto('/')`
    baseURL,

    // Collect trace when retrying failed tests
    trace: 'on-first-retry',

    // Capture screenshot on failure
    screenshot: 'only-on-failure',

    // Record video on failure
    video: 'on-first-retry',

    // Viewport size for consistent screenshots
    viewport: { width: 1280, height: 720 },
  },

  // Configure projects for major browsers
  projects: [
    // Windows WebView2 (Chromium-based)
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // High device scale factor for Retina-quality screenshots
        deviceScaleFactor: 2,
      },
    },

    // Firefox for cross-browser compatibility
    {
      name: 'firefox',
      use: {
        ...devices['Desktop Firefox'],
        deviceScaleFactor: 2,
      },
    },

    // WebKit for Safari compatibility
    {
      name: 'webkit',
      use: {
        ...devices['Desktop Safari'],
        deviceScaleFactor: 2,
      },
    },

    // Visual regression specific project
    {
      name: 'visual-regression',
      use: {
        ...devices['Desktop Chrome'],
        deviceScaleFactor: 2,
        // Disable animations for consistent screenshots
        contextOptions: {
          reducedMotion: 'reduce',
        },
      },
      testMatch: /visual\/.*\.spec\.ts/,
    },

    // Accessibility testing project
    {
      name: 'accessibility',
      use: {
        ...devices['Desktop Chrome'],
        deviceScaleFactor: 1,
      },
      testMatch: /a11y\/.*\.spec\.ts/,
    },
  ],

  // Run local dev server before starting tests
  webServer: process.env.TEST_URL ? undefined : {
    command: 'cargo run --package easyssh-winui',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },

  // Snapshot configuration for visual regression
  snapshotDir: 'tests/visual/__snapshots__',
  snapshotPathTemplate: '{snapshotDir}/{projectName}/{testFilePath}/{arg}{ext}',

  // Expect configuration
  expect: {
    // Maximum time expect() should wait for the condition to be met
    timeout: 5000,

    // Threshold for pixel comparison (0.2 = 0.2% difference allowed)
    toHaveScreenshot: {
      maxDiffPixels: 100,
      threshold: 0.2,
      animations: 'disabled',
    },

    // Threshold for visual comparison
    toMatchSnapshot: {
      threshold: 0.2,
    },
  },
});
