import { chromium, FullConfig } from '@playwright/test';

async function globalSetup(config: FullConfig) {
  // Start browser for authentication or global setup
  const browser = await chromium.launch();
  const context = await browser.newContext();
  const page = await context.newPage();

  console.log('🚀 Starting global setup for pH7Console tests');

  try {
    // Wait for the application to be available
    await page.goto(config.projects[0].use.baseURL || 'http://localhost:1420', {
      waitUntil: 'networkidle',
      timeout: 60000,
    });

    // Verify core components are loaded
    await page.waitForSelector('[data-testid="terminal-container"]', { timeout: 30000 });
    
    console.log('✅ Application is ready for testing');
  } catch (error) {
    console.error('❌ Failed to setup application:', error);
    throw error;
  } finally {
    await context.close();
    await browser.close();
  }
}

export default globalSetup;
