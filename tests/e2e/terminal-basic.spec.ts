import { test, expect } from '../setup';
import { TEST_CONFIG } from '../setup';

test.describe('pH7Console - Basic Terminal Functionality', () => {
  test.beforeEach(async ({ appFixture }) => {
    await appFixture.goto();
  });

  test('should load the application successfully', async ({ appFixture }) => {
    await expect(appFixture.page).toHaveTitle(/pH7Console/);
    await expect(appFixture.terminalInput).toBeVisible();
    await expect(appFixture.sidebar).toBeVisible();
  });

  test('should execute basic commands', async ({ appFixture }) => {
    // Test basic commands
    for (const command of TEST_CONFIG.testCommands.slice(0, 3)) {
      await appFixture.executeCommand(command);
      await appFixture.waitForCommandCompletion();
      
      const output = await appFixture.getTerminalOutput();
      expect(output).toBeDefined();
      expect(output).not.toBe('');
    }
  });

  test('should show command history', async ({ appFixture }) => {
    // Execute a command first
    await appFixture.executeCommand('pwd');
    await appFixture.waitForCommandCompletion();
    
    // Test command history
    const historyCommand = await appFixture.getCommandHistory();
    expect(historyCommand).toBe('pwd');
  });

  test('should handle invalid commands gracefully', async ({ appFixture }) => {
    await appFixture.executeCommand('invalidcommandthatdoesnotexist123');
    await appFixture.waitForCommandCompletion();
    
    const output = await appFixture.getTerminalOutput();
    expect(output).toContain('command not found');
  });

  test('should support multiple terminal sessions', async ({ appFixture }) => {
    // Create a new terminal
    await appFixture.createNewTerminal();
    
    // Should have at least 2 terminal tabs
    const terminalTabs = await appFixture.sidebar.locator('[data-testid^="terminal-tab-"]').count();
    expect(terminalTabs).toBeGreaterThanOrEqual(2);
  });

  test('should persist terminal state when switching sessions', async ({ appFixture }) => {
    // Execute command in first terminal
    await appFixture.executeCommand('echo "first terminal"');
    await appFixture.waitForCommandCompletion();
    
    // Create and switch to new terminal
    await appFixture.createNewTerminal();
    await appFixture.executeCommand('echo "second terminal"');
    await appFixture.waitForCommandCompletion();
    
    // Switch back to first terminal (assuming it has ID 0 or 1)
    const firstTerminalTab = appFixture.sidebar.locator('[data-testid^="terminal-tab-"]').first();
    await firstTerminalTab.click();
    
    // Check that first terminal's output is still there
    const output = await appFixture.getTerminalOutput();
    expect(output).toContain('first terminal');
  });
});
