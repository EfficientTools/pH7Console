// Test setup configuration for pH7Console E2E tests
import { test as base, expect } from '@playwright/test';
import { AppFixture } from './fixtures/AppFixture';

// Extend the base test with our custom fixtures
export const test = base.extend<{ appFixture: AppFixture }>({
  appFixture: async ({ page }, use) => {
    const appFixture = new AppFixture(page);
    await use(appFixture);
  },
});

export { expect };

// Global test configuration
export const TEST_CONFIG = {
  // Timeout for individual tests
  timeout: 30000,
  
  // Retries for flaky tests
  retries: 2,
  
  // Test artifacts
  screenshotOnFailure: true,
  videoOnFailure: true,
  
  // Tauri app configuration
  tauriApp: {
    buildCommand: 'npm run tauri build',
    devCommand: 'npm run tauri dev',
    distDir: '../dist',
    devPath: 'http://localhost:5173',
  },
  
  // Test data
  testCommands: [
    'ls -la',
    'pwd',
    'echo "test"',
    'cd ..',
    'git status',
    'npm --version',
    'node --version',
    'which git',
  ],
  
  // AI features to test
  aiFeatures: {
    naturalLanguageCommands: [
      'show me large files',
      'list recent files',
      'check system resources',
      'find javascript files',
    ],
    commandValidation: [
      'ls /nonexistent',
      'invalidcommand',
      'git status',
      'npm list',
    ],
  },
};
