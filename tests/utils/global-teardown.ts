import { FullConfig } from '@playwright/test';

/**
 * Global Teardown for Playwright Tests
 *
 * Runs once after all tests to clean up the environment.
 */
async function globalTeardown(config: FullConfig) {
  const isCI = !!process.env.CI;

  console.log('=== Playwright Global Teardown ===');

  // Clean up test artifacts if needed
  if (process.env.CLEANUP_TEST_DATA === 'true') {
    console.log('Cleaning up test data...');
    // Add cleanup logic here if needed
  }

  // Report summary
  if (isCI) {
    console.log('Test run completed in CI environment');
    console.log('Reports available at: playwright-report/');
  }

  console.log('Global teardown completed');
}

export default globalTeardown;
