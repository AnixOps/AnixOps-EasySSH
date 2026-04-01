import { FullConfig } from '@playwright/test';

/**
 * Global Setup for Playwright Tests
 *
 * Runs once before all tests to set up the environment.
 */
async function globalSetup(config: FullConfig) {
  // Detect environment
  const isCI = !!process.env.CI;
  const isGitHubActions = !!process.env.GITHUB_ACTIONS;

  console.log('=== Playwright Global Setup ===');
  console.log(`Environment: ${isCI ? 'CI' : 'Local'}`);
  console.log(`GitHub Actions: ${isGitHubActions ? 'Yes' : 'No'}`);
  console.log(`Base URL: ${config.projects[0]?.use?.baseURL || 'Not set'}`);

  // Verify Tauri development server is accessible (optional check)
  if (process.env.VERIFY_TAURI_SERVER === 'true') {
    try {
      const response = await fetch(config.projects[0]?.use?.baseURL || 'http://localhost:1420');
      if (response.ok) {
        console.log('Tauri development server is accessible');
      }
    } catch (error) {
      console.warn('Warning: Tauri development server may not be ready');
    }
  }

  // Set up test environment variables
  process.env.TEST_ENVIRONMENT = 'playwright';
  process.env.TEST_TIMESTAMP = Date.now().toString();

  console.log('Global setup completed');
}

export default globalSetup;
