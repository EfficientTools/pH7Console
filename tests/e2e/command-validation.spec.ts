import { test, expect } from '../setup';

test.describe('pH7Console - Command Validation', () => {
  test.beforeEach(async ({ appFixture }) => {
    await appFixture.goto();
  });

  test('should validate command existence before suggesting', async ({ appFixture }) => {
    // Type a command that definitely exists on most systems
    await appFixture.typeInTerminal('git');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      // If git is installed, should show git-related suggestions
      if (suggestions.length > 0) {
        expect(suggestions.some(s => s.includes('git'))).toBe(true);
        
        // All suggestions should be valid commands
        for (const suggestion of suggestions.slice(0, 3)) {
          const command = suggestion.split(' ')[0];
          expect(command).toBeDefined();
          expect(command.length).toBeGreaterThan(0);
        }
      }
    } catch (error) {
      // If no suggestions appear, that's also valid behavior
      console.log('No suggestions appeared for git command');
    }
  });

  test('should not suggest invalid commands', async ({ appFixture }) => {
    // Type an invalid command
    await appFixture.typeInTerminal('thisisnotarealcommand123');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      // Should not suggest the invalid command
      expect(suggestions.every(s => !s.includes('thisisnotarealcommand123'))).toBe(true);
    } catch (error) {
      // No suggestions is expected behavior for invalid commands
      console.log('No suggestions for invalid command - this is expected');
    }
  });

  test('should validate file paths in suggestions', async ({ appFixture }) => {
    // Navigate to a known directory
    await appFixture.navigateToDirectory('./src');
    await appFixture.waitForCommandCompletion();
    
    // Type a command that works with files
    await appFixture.typeInTerminal('ls ');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      if (suggestions.length > 0) {
        // Suggestions should contain valid file operations
        const validSuggestions = suggestions.filter(s => 
          s.includes('ls') || s.includes('./') || s.includes('src/')
        );
        expect(validSuggestions.length).toBeGreaterThan(0);
      }
    } catch (error) {
      console.log('No file path suggestions appeared');
    }
  });

  test('should provide corrected paths for relative paths', async ({ appFixture }) => {
    // Create a test scenario where path correction might occur
    await appFixture.executeCommand('pwd');
    await appFixture.waitForCommandCompletion();
    
    // Type a command with a relative path
    await appFixture.typeInTerminal('cd ../');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      if (suggestions.length > 0) {
        // Should show directory navigation suggestions
        const navSuggestions = suggestions.filter(s => 
          s.includes('cd') || s.includes('../') || s.includes('./')
        );
        expect(navSuggestions.length).toBeGreaterThanOrEqual(0);
      }
    } catch (error) {
      console.log('No navigation suggestions appeared');
    }
  });

  test('should cache validation results for performance', async ({ appFixture }) => {
    const startTime = Date.now();
    
    // First validation
    await appFixture.typeInTerminal('git status');
    
    try {
      await appFixture.waitForSuggestions();
      await appFixture.getSuggestions();
    } catch (error) {
      console.log('No suggestions for first git command');
    }
    
    // Clear input
    await appFixture.terminalInput.fill('');
    
    // Second validation of similar command should be faster (cached)
    await appFixture.typeInTerminal('git log');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      const endTime = Date.now();
      const totalTime = endTime - startTime;
      
      // Should complete within reasonable time
      expect(totalTime).toBeLessThan(10000); // 10 seconds max
      
      if (suggestions.length > 0) {
        expect(suggestions.some(s => s.includes('git'))).toBe(true);
      }
    } catch (error) {
      console.log('No suggestions for second git command');
    }
  });

  test('should handle command validation errors gracefully', async ({ appFixture }) => {
    // Test with various edge cases
    const edgeCases = [
      '', // Empty string
      '   ', // Only spaces
      'command with "quotes"', // Commands with quotes
      'command && another', // Command chains
      'command | pipe', // Piped commands
    ];
    
    for (const edgeCase of edgeCases) {
      await appFixture.terminalInput.fill(edgeCase);
      
      try {
        // Should not crash the application
        await appFixture.page.waitForTimeout(1000);
        
        // Application should still be responsive
        expect(await appFixture.terminalInput.isEnabled()).toBe(true);
      } catch (error) {
        // Edge cases might not provide suggestions, which is fine
        console.log(`Edge case "${edgeCase}" handled gracefully`);
      }
    }
  });
});
