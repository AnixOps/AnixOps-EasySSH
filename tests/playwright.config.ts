import { defineConfig, devices } from '@playwright/test';
import path from 'path';

/**
 * Playwright Configuration for EasySSH
 *
 * Projects:
 * - chromium: Chrome/Edge testing
 * - firefox: Firefox testing
 * - webkit: Safari testing
 * - visual-regression: Screenshot comparison tests
 * - accessibility: Axe-core WCAG compliance tests
 */

// Detect CI environment
const isCI = !!process.env.CI;
const isGitHubActions = !!process.env.GITHUB_ACTIONS;

// Tauri development server configuration
const TAURI_DEV_SERVER = 'http://localhost:1420';
const TAURI_WEBVIEW_SERVER = 'http://localhost:3000';

export default defineConfig({
  // Test directory
  testDir: path.join(__dirname, '.'),

  // Run files matching these patterns
  testMatch: [
    'e2e/**/*.spec.ts',
    'visual/**/*.spec.ts',
    'a11y/**/*.spec.ts',
    'integration/**/*.spec.ts',
  ],

  // Timeout settings
  timeout: isCI ? 60000 : 30000,
  expect: {
    timeout: isCI ? 10000 : 5000,
  },

  // Retry configuration
  retries: isCI ? 2 : 1,

  // Workers - reduce in CI for stability
  workers: isCI ? 2 : 4,

  // Fail fast in CI to save resources
  maxFailures: isCI ? 5 : 0,

  // Reporter configuration
  reporter: [
    ['list'],
    ['html', { open: isCI ? 'never' : 'on-failure', outputFolder: 'playwright-report' }],
    ['json', { outputFile: 'playwright-report/results.json' }],
    ...(isGitHubActions ? [['github'] as [string]] : []),
    ...(isCI ? [['junit', { outputFile: 'playwright-report/junit.xml' }] as [string, { outputFile: string }]] : []),
  ],

  // Global setup/teardown
  globalSetup: require.resolve('./utils/global-setup.ts'),
  globalTeardown: require.resolve('./utils/global-teardown.ts'),

  // Shared settings for all projects
  use: {
    // Base URL for all navigations
    baseURL: process.env.PLAYWRIGHT_BASE_URL || TAURI_DEV_SERVER,

    // Capture screenshots on failure
    screenshot: 'only-on-failure',

    // Record video on failure (disabled in CI to save resources)
    video: isCI ? 'off' : 'on-first-retry',

    // Record traces on failure
    trace: 'on-first-retry',

    // Action timeout
    actionTimeout: 15000,

    // Navigation timeout
    navigationTimeout: 20000,

    // Viewport settings
    viewport: { width: 1440, height: 900 },

    // Device scale factor for retina displays
    deviceScaleFactor: 1,

    // Ignore HTTPS errors (for local development)
    ignoreHTTPSErrors: true,

    // Launch options
    launchOptions: {
      slowMo: process.env.PLAYWRIGHT_SLOWMO ? parseInt(process.env.PLAYWRIGHT_SLOWMO, 10) : 0,
    },
  },

  // Project configurations
  projects: [
    // Chromium - Primary browser for testing
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        channel: 'chrome',
      },
      testIgnore: ['**/visual/**/*.spec.ts', '**/a11y/**/*.spec.ts'],
    },

    // Firefox - Cross-browser compatibility
    {
      name: 'firefox',
      use: {
        ...devices['Desktop Firefox'],
      },
      testIgnore: ['**/visual/**/*.spec.ts', '**/a11y/**/*.spec.ts'],
    },

    // WebKit - Safari compatibility
    {
      name: 'webkit',
      use: {
        ...devices['Desktop Safari'],
      },
      testIgnore: ['**/visual/**/*.spec.ts', '**/a11y/**/*.spec.ts'],
    },

    // Visual Regression - Screenshot testing
    {
      name: 'visual-regression',
      use: {
        ...devices['Desktop Chrome'],
        // Consistent viewport for screenshots
        viewport: { width: 1920, height: 1080 },
      },
      testMatch: '**/visual/**/*.spec.ts',
      // Retry more for visual tests (flaky due to rendering differences)
      retries: isCI ? 3 : 2,
      // Visual comparison options
      expect: {
        timeout: 15000,
        toHaveScreenshot: {
          maxDiffPixels: 100,
          threshold: 0.2,
          animations: 'disabled',
          scale: 'css',
        },
      },
    },

    // Accessibility - WCAG compliance testing
    {
      name: 'accessibility',
      use: {
        ...devices['Desktop Chrome'],
      },
      testMatch: '**/a11y/**/*.spec.ts',
      // Accessibility tests should be deterministic
      retries: isCI ? 1 : 0,
    },

    // Mobile Chrome - Responsive testing (optional)
    {
      name: 'Mobile Chrome',
      use: { ...devices['Pixel 5'] },
      testMatch: '**/e2e/**/*.spec.ts',
      dependencies: ['chromium'],
    },

    // Tablet Safari - iPad testing (optional)
    {
      name: 'Tablet Safari',
      use: { ...devices['iPad Pro 11'] },
      testMatch: '**/e2e/**/*.spec.ts',
      dependencies: ['webkit'],
    },
  ],

  // Development server configuration for Tauri
  webServer: [
    // Vite dev server (frontend)
    {
      command: 'cd .. && npm run dev',
      url: TAURI_DEV_SERVER,
      timeout: 120000,
      reuseExistingServer: !isCI,
      stdout: 'pipe',
      stderr: 'pipe',
    },
  ],

  // Output directory for test artifacts
  outputDir: 'test-results/',

  // Snapshot directory configuration
  snapshotDir: 'snapshots/',

  // Update snapshots in CI only when explicitly requested
  updateSnapshots: process.env.UPDATE_SNAPSHOTS === 'true' ? 'all' : 'none',

  // Fully parallel mode - run all tests in parallel
  fullyParallel: true,

  // Forbid test.only in CI
  forbidOnly: isCI,

  // Preserve output for debugging
  preserveOutput: 'failures-only',
});
