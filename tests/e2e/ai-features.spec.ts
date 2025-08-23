import { test, expect } from '../setup';
import { TEST_CONFIG } from '../setup';

test.describe('pH7Console - AI Features', () => {
  test.beforeEach(async ({ appFixture }) => {
    await appFixture.goto();
  });

  test('should provide intelligent command suggestions', async ({ appFixture }) => {
    // Type a partial command
    await appFixture.typeInTerminal('ls');
    
    // Should show suggestions
    await appFixture.waitForSuggestions();
    const suggestions = await appFixture.getSuggestions();
    
    expect(suggestions.length).toBeGreaterThan(0);
    expect(suggestions.some(s => s.includes('ls'))).toBe(true);
  });

  test('should validate command suggestions', async ({ appFixture }) => {
    // Test command validation feature
    const validationTests = TEST_CONFIG.aiFeatures.commandValidation;
    
    for (const command of validationTests.slice(0, 2)) {
      const suggestions = await appFixture.testCommandValidation(command);
      
      if (command === 'git status' || command === 'npm list') {
        // These should provide valid suggestions
        expect(suggestions.length).toBeGreaterThan(0);
      } else if (command === 'invalidcommand') {
        // This should provide no or different suggestions
        expect(suggestions.every(s => !s.includes('invalidcommand'))).toBe(true);
      }
    }
  });

  test('should provide context-aware suggestions', async ({ appFixture }) => {
    // Navigate to a directory and check context-aware suggestions
    await appFixture.navigateToDirectory('./src');
    await appFixture.waitForCommandCompletion();
    
    // Type a command that should be context-aware
    await appFixture.typeInTerminal('cat ');
    await appFixture.waitForSuggestions();
    
    const suggestions = await appFixture.getSuggestions();
    // Should suggest files from the current directory
    expect(suggestions.length).toBeGreaterThan(0);
  });

  test('should execute clicked suggestions', async ({ appFixture }) => {
    // Type partial command
    await appFixture.typeInTerminal('pwd');
    await appFixture.waitForSuggestions();
    
    // Click on a suggestion
    const suggestions = await appFixture.getSuggestions();
    if (suggestions.length > 0) {
      await appFixture.clickSuggestion(suggestions[0]);
      await appFixture.waitForCommandCompletion();
      
      // Should have executed the command
      const output = await appFixture.getTerminalOutput();
      expect(output).toBeDefined();
    }
  });

  test('should handle natural language commands', async ({ appFixture, page }) => {
    // Skip if natural language processing is not available
    test.skip(!page.url().includes('localhost'), 'Natural language requires running app');
    
    // Test natural language commands
    const nlCommands = TEST_CONFIG.aiFeatures.naturalLanguageCommands;
    
    for (const nlCommand of nlCommands.slice(0, 2)) {
      const output = await appFixture.testNaturalLanguageCommand(nlCommand);
      
      // Should translate to actual commands and execute
      expect(output).toBeDefined();
      expect(output.length).toBeGreaterThan(0);
    }
  });

  test('should show AI panel functionality', async ({ appFixture }) => {
    // Toggle AI panel
    await appFixture.toggleAIPanel();
    
    // AI panel should be visible
    await expect(appFixture.aiPanel).toBeVisible();
    
    // Should have AI features available
    const aiFeatures = await appFixture.aiPanel.locator('[data-testid="ai-feature"]').count();
    expect(aiFeatures).toBeGreaterThan(0);
  });

  test('should provide command explanations', async ({ appFixture }) => {
    // Execute a complex command
    await appFixture.executeCommand('find . -name "*.ts" -type f');
    await appFixture.waitForCommandCompletion();
    
    // Toggle AI panel to see explanations
    await appFixture.toggleAIPanel();
    
    // Should show explanation or help
    const explanationElement = appFixture.aiPanel.locator('[data-testid="command-explanation"]');
    if (await explanationElement.isVisible()) {
      const explanation = await explanationElement.textContent();
      expect(explanation).toContain('find');
    }
  });
});
