import { test, expect } from '@playwright/test';

test.describe('Terminal Workflow E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the application
    await page.goto('/');
    
    // Wait for the terminal to be ready
    await page.waitForSelector('[data-testid="terminal-container"]', { timeout: 10000 });
    
    // Ensure the terminal input is visible and focused
    await page.waitForSelector('[data-testid="terminal-input"]');
  });

  test('should render terminal interface correctly', async ({ page }) => {
    // Check main terminal elements are present
    await expect(page.locator('[data-testid="terminal-container"]')).toBeVisible();
    await expect(page.locator('[data-testid="terminal-header"]')).toBeVisible();
    await expect(page.locator('[data-testid="terminal-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="terminal-output"]')).toBeVisible();
    
    // Check terminal header elements
    await expect(page.locator('[data-testid="file-explorer-button"]')).toBeVisible();
    await expect(page.locator('[data-testid="ai-assistant-toggle"]')).toBeVisible();
    await expect(page.locator('[data-testid="settings-button"]')).toBeVisible();
  });

  test('should execute basic commands successfully', async ({ page }) => {
    const commands = [
      { cmd: 'pwd', expectOutput: true },
      { cmd: 'ls', expectOutput: true },
      { cmd: 'echo "Hello World"', expectOutput: 'Hello World' },
      { cmd: 'date', expectOutput: true },
    ];

    for (const { cmd, expectOutput } of commands) {
      // Clear any previous output
      await page.locator('[data-testid="terminal-input"]').fill('');
      
      // Type command
      await page.locator('[data-testid="terminal-input"]').fill(cmd);
      
      // Press Enter
      await page.keyboard.press('Enter');
      
      // Wait for command execution
      await page.waitForTimeout(1000);
      
      // Check output
      const output = page.locator('[data-testid="terminal-output"]');
      if (typeof expectOutput === 'string') {
        await expect(output).toContainText(expectOutput);
      } else if (expectOutput) {
        await expect(output).not.toBeEmpty();
      }
      
      // Verify prompt is ready for next command
      await expect(page.locator('[data-testid="terminal-input"]')).toBeVisible();
    }
  });

  test('should handle command history navigation', async ({ page }) => {
    const commands = ['echo "first"', 'echo "second"', 'echo "third"'];
    
    // Execute multiple commands
    for (const cmd of commands) {
      await page.locator('[data-testid="terminal-input"]').fill(cmd);
      await page.keyboard.press('Enter');
      await page.waitForTimeout(500);
    }
    
    // Clear input
    await page.locator('[data-testid="terminal-input"]').fill('');
    
    // Navigate history with up arrow
    await page.keyboard.press('ArrowUp');
    await expect(page.locator('[data-testid="terminal-input"]')).toHaveValue('echo "third"');
    
    await page.keyboard.press('ArrowUp');
    await expect(page.locator('[data-testid="terminal-input"]')).toHaveValue('echo "second"');
    
    await page.keyboard.press('ArrowUp');
    await expect(page.locator('[data-testid="terminal-input"]')).toHaveValue('echo "first"');
    
    // Navigate forward with down arrow
    await page.keyboard.press('ArrowDown');
    await expect(page.locator('[data-testid="terminal-input"]')).toHaveValue('echo "second"');
  });

  test('should toggle AI assistant successfully', async ({ page }) => {
    const aiToggle = page.locator('[data-testid="ai-assistant-toggle"]');
    
    // Check initial state
    await expect(aiToggle).toBeVisible();
    
    // Click to enable AI assistant
    await aiToggle.click();
    
    // Verify AI suggestions appear
    await expect(page.locator('[data-testid="ai-suggestions"]')).toBeVisible();
    
    // Test AI command suggestion
    await page.locator('[data-testid="terminal-input"]').fill('show me large files');
    await page.waitForTimeout(500);
    
    // Should show AI suggestion
    await expect(page.locator('[data-testid="ai-suggestion-item"]')).toBeVisible();
    
    // Toggle off AI assistant
    await aiToggle.click();
    await expect(page.locator('[data-testid="ai-suggestions"]')).not.toBeVisible();
  });

  test('should open and navigate file explorer', async ({ page }) => {
    const fileExplorerButton = page.locator('[data-testid="file-explorer-button"]');
    
    // Open file explorer
    await fileExplorerButton.click();
    
    // Verify file explorer is visible
    await expect(page.locator('[data-testid="file-explorer"]')).toBeVisible();
    await expect(page.locator('[data-testid="file-tree"]')).toBeVisible();
    
    // Check breadcrumbs
    await expect(page.locator('[data-testid="breadcrumbs"]')).toBeVisible();
    
    // Check directory listing
    const fileItems = page.locator('[data-testid="file-item"]');
    await expect(fileItems.first()).toBeVisible();
    
    // Test directory navigation (if directories exist)
    const directories = page.locator('[data-testid="file-item"][data-type="directory"]');
    const dirCount = await directories.count();
    
    if (dirCount > 0) {
      await directories.first().dblclick();
      
      // Wait for navigation
      await page.waitForTimeout(500);
      
      // Check that breadcrumbs updated
      await expect(page.locator('[data-testid="breadcrumbs"]')).toContainText('/');
    }
  });

  test('should handle file operations in explorer', async ({ page }) => {
    // Open file explorer
    await page.locator('[data-testid="file-explorer-button"]').click();
    await expect(page.locator('[data-testid="file-explorer"]')).toBeVisible();
    
    // Test right-click context menu
    const fileItem = page.locator('[data-testid="file-item"]').first();
    await fileItem.click({ button: 'right' });
    
    // Verify context menu appears
    await expect(page.locator('[data-testid="context-menu"]')).toBeVisible();
    
    // Check context menu options
    await expect(page.locator('[data-testid="context-menu-copy"]')).toBeVisible();
    await expect(page.locator('[data-testid="context-menu-rename"]')).toBeVisible();
    await expect(page.locator('[data-testid="context-menu-delete"]')).toBeVisible();
    
    // Close context menu
    await page.keyboard.press('Escape');
    await expect(page.locator('[data-testid="context-menu"]')).not.toBeVisible();
  });

  test('should handle search functionality', async ({ page }) => {
    // Open file explorer
    await page.locator('[data-testid="file-explorer-button"]').click();
    
    // Find and use search input
    const searchInput = page.locator('[data-testid="search-input"]');
    await expect(searchInput).toBeVisible();
    
    // Test search
    await searchInput.fill('*.ts');
    await page.keyboard.press('Enter');
    
    // Wait for search results
    await page.waitForTimeout(1000);
    
    // Verify search results
    const searchResults = page.locator('[data-testid="search-results"]');
    await expect(searchResults).toBeVisible();
    
    // Clear search
    await searchInput.fill('');
    await page.keyboard.press('Enter');
    
    // Verify normal file view returns
    await expect(page.locator('[data-testid="file-tree"]')).toBeVisible();
  });

  test('should handle keyboard shortcuts', async ({ page }) => {
    // Test Ctrl+L to clear terminal
    await page.locator('[data-testid="terminal-input"]').fill('echo "test"');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
    
    // Clear terminal with shortcut
    await page.keyboard.press('Control+l');
    
    // Verify terminal is cleared
    const output = page.locator('[data-testid="terminal-output"]');
    await expect(output).toBeEmpty();
    
    // Test Ctrl+C to interrupt command
    await page.locator('[data-testid="terminal-input"]').fill('sleep 10');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
    
    // Interrupt with Ctrl+C
    await page.keyboard.press('Control+c');
    
    // Verify prompt is available
    await expect(page.locator('[data-testid="terminal-input"]')).toBeVisible();
  });

  test('should handle error scenarios gracefully', async ({ page }) => {
    const errorCommands = [
      'invalidcommand',
      'ls /nonexistent/path',
      'cat /dev/null/impossible',
    ];

    for (const cmd of errorCommands) {
      await page.locator('[data-testid="terminal-input"]').fill(cmd);
      await page.keyboard.press('Enter');
      await page.waitForTimeout(1000);
      
      // Verify error is displayed but terminal remains functional
      const output = page.locator('[data-testid="terminal-output"]');
      await expect(output).toContainText(/error|not found|no such|command not found/i);
      
      // Verify prompt is still available
      await expect(page.locator('[data-testid="terminal-input"]')).toBeVisible();
      await expect(page.locator('[data-testid="terminal-input"]')).toHaveValue('');
    }
  });

  test('should persist session state across interactions', async ({ page }) => {
    // Execute commands that change state
    await page.locator('[data-testid="terminal-input"]').fill('export TEST_VAR="hello"');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
    
    // Create a test file
    await page.locator('[data-testid="terminal-input"]').fill('echo "test content" > test_file.txt');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
    
    // Verify environment variable persists
    await page.locator('[data-testid="terminal-input"]').fill('echo $TEST_VAR');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
    
    const output = page.locator('[data-testid="terminal-output"]');
    await expect(output).toContainText('hello');
    
    // Verify file was created
    await page.locator('[data-testid="terminal-input"]').fill('cat test_file.txt');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
    
    await expect(output).toContainText('test content');
    
    // Cleanup
    await page.locator('[data-testid="terminal-input"]').fill('rm test_file.txt');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
  });

  test('should handle AI natural language commands', async ({ page }) => {
    // Enable AI assistant
    await page.locator('[data-testid="ai-assistant-toggle"]').click();
    
    const naturalLanguageCommands = [
      { input: 'show me all javascript files', expectedCommand: 'find . -name "*.js"' },
      { input: 'list large files', expectedCommand: 'find . -size +1M' },
      { input: 'check disk usage', expectedCommand: 'df -h' },
      { input: 'show running processes', expectedCommand: 'ps aux' },
    ];

    for (const { input, expectedCommand } of naturalLanguageCommands) {
      await page.locator('[data-testid="terminal-input"]').fill(input);
      await page.waitForTimeout(1000);
      
      // Verify AI suggestion appears
      const suggestion = page.locator('[data-testid="ai-suggestion-item"]').first();
      await expect(suggestion).toBeVisible();
      await expect(suggestion).toContainText(expectedCommand);
      
      // Accept suggestion
      await suggestion.click();
      
      // Verify command is executed
      await expect(page.locator('[data-testid="terminal-input"]')).toHaveValue(expectedCommand);
      
      // Execute the command
      await page.keyboard.press('Enter');
      await page.waitForTimeout(1000);
      
      // Clear for next test
      await page.locator('[data-testid="terminal-input"]').fill('');
    }
  });
});
