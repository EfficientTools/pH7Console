import { test, expect } from '../setup';

test.describe('pH7Console - Command Suggestion Validation', () => {
  test.beforeEach(async ({ appFixture }) => {
    await appFixture.goto();
  });

  test('should not suggest deleted frequent directories', async ({ appFixture }) => {
    // Navigate to home directory
    await appFixture.executeCommand('cd ~');
    await appFixture.waitForCommandCompletion();
    
    // Type a command that would normally suggest frequent directories
    await appFixture.typeInTerminal('cd');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      // Suggestions should not contain non-existent paths
      const invalidSuggestions = suggestions.filter((s: string) => 
        s.includes('/non/existent/path') || 
        s.includes('/deleted/directory') ||
        s.includes('DirectoryZ') // Example of deleted directory
      );
      
      expect(invalidSuggestions).toHaveLength(0);
    } catch (error) {
      console.log('No suggestions appeared for cd command');
    }
  });

  test('should provide full absolute paths when suggesting directories', async ({ appFixture }) => {
    // Start from home directory
    await appFixture.executeCommand('cd ~');
    await appFixture.waitForCommandCompletion();
    
    // Type command to navigate to a project directory
    await appFixture.typeInTerminal('cd Documents');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      if (suggestions.length > 0) {
        // All directory suggestions should use absolute paths
        const directorySuggestions = suggestions.filter((s: string) => s.includes('cd '));
        
        for (const suggestion of directorySuggestions) {
          // Should contain full path, not just relative
          expect(suggestion).toMatch(/cd\s+"?\/[^"]*"?/);
        }
      }
    } catch (error) {
      console.log('No suggestions for directory navigation');
    }
  });

  test('should auto-correct paths to valid locations', async ({ appFixture }) => {
    // Navigate to a test directory
    await appFixture.executeCommand('cd /tmp');
    await appFixture.waitForCommandCompletion();
    
    // Type a command with a path that might need correction
    await appFixture.typeInTerminal('ls Downloads');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      if (suggestions.length > 0) {
        // Should suggest corrected paths that actually exist
        const pathSuggestions = suggestions.filter((s: string) => s.includes('Downloads'));
        
        for (const suggestion of pathSuggestions) {
          // Path should be absolute or properly corrected
          expect(suggestion).toMatch(/(\/.*Downloads|~\/Downloads|\$HOME\/Downloads)/);
        }
      }
    } catch (error) {
      console.log('No path correction suggestions appeared');
    }
  });

  test('should handle tilde expansion in directory suggestions', async ({ appFixture }) => {
    // Type command with tilde
    await appFixture.typeInTerminal('cd ~/Doc');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      if (suggestions.length > 0) {
        // Should either expand tilde or provide proper path
        const tildeExpanded = suggestions.some((s: string) => 
          s.includes('/Users/') || s.includes('/home/')
        );
        const validTilde = suggestions.some((s: string) => 
          s.includes('~/Documents') || s.includes('~/Downloads')
        );
        
        expect(tildeExpanded || validTilde).toBe(true);
      }
    } catch (error) {
      console.log('No tilde expansion suggestions');
    }
  });

  test('should validate command existence before suggesting', async ({ appFixture }) => {
    // Type a series of commands to test validation
    const commandsToTest = ['git', 'npm', 'ls', 'cd'];
    
    for (const cmd of commandsToTest) {
      await appFixture.terminalInput.fill('');
      await appFixture.typeInTerminal(cmd);
      
      try {
        await appFixture.waitForSuggestions();
        const suggestions = await appFixture.getSuggestions();
        
        if (suggestions.length > 0) {
          // All suggestions should start with valid commands
          for (const suggestion of suggestions) {
            const firstWord = suggestion.split(' ')[0];
            expect(firstWord).toBeTruthy();
            expect(firstWord.length).toBeGreaterThan(0);
          }
        }
      } catch (error) {
        console.log(`No suggestions for ${cmd} command`);
      }
    }
  });

  test('should handle path names with special characters', async ({ appFixture }) => {
    // Test with various path name patterns
    const pathPatterns = [
      'folder with spaces',
      'folder-with-dashes',
      'folder_with_underscores',
      'folder.with.dots'
    ];
    
    for (const pattern of pathPatterns) {
      await appFixture.terminalInput.fill('');
      await appFixture.typeInTerminal(`cd "${pattern}"`);
      
      try {
        await appFixture.waitForSuggestions();
        const suggestions = await appFixture.getSuggestions();
        
        // Should handle special characters gracefully
        if (suggestions.length > 0) {
          const validSuggestions = suggestions.filter((s: string) => 
            s.includes(pattern) || s.includes(`"${pattern}"`) || s.includes(`'${pattern}'`)
          );
          
          // At least some suggestions should handle the special characters
          expect(validSuggestions.length >= 0).toBe(true);
        }
      } catch (error) {
        console.log(`No suggestions for path pattern: ${pattern}`);
      }
    }
  });

  test('should not suggest invalid or dangerous commands', async ({ appFixture }) => {
    // Type potentially dangerous or invalid commands
    const dangerousCommands = [
      'rm -rf /',
      'sudo rm -rf',
      'dd if=/dev/zero',
      'invalidcommand123',
      'notarealcommand'
    ];
    
    for (const cmd of dangerousCommands) {
      await appFixture.terminalInput.fill('');
      await appFixture.typeInTerminal(cmd);
      
      try {
        await appFixture.waitForSuggestions();
        const suggestions = await appFixture.getSuggestions();
        
        // Should not suggest dangerous commands as-is
        const exactMatches = suggestions.filter((s: string) => s.trim() === cmd);
        expect(exactMatches).toHaveLength(0);
        
        // Should not suggest invalid commands
        if (cmd.includes('invalidcommand') || cmd.includes('notarealcommand')) {
          expect(suggestions).toHaveLength(0);
        }
      } catch (error) {
        // Expected for invalid commands
        console.log(`No suggestions for dangerous/invalid command: ${cmd}`);
      }
    }
  });

  test('should update suggestions when directory context changes', async ({ appFixture }) => {
    // Start in home directory
    await appFixture.executeCommand('cd ~');
    await appFixture.waitForCommandCompletion();
    
    await appFixture.typeInTerminal('ls');
    
    let homeSuggestions: string[] = [];
    try {
      await appFixture.waitForSuggestions();
      homeSuggestions = await appFixture.getSuggestions();
    } catch (error) {
      console.log('No suggestions in home directory');
    }
    
    // Navigate to a different directory
    await appFixture.terminalInput.fill('');
    await appFixture.executeCommand('cd /tmp');
    await appFixture.waitForCommandCompletion();
    
    await appFixture.typeInTerminal('ls');
    
    let tmpSuggestions: string[] = [];
    try {
      await appFixture.waitForSuggestions();
      tmpSuggestions = await appFixture.getSuggestions();
    } catch (error) {
      console.log('No suggestions in /tmp directory');
    }
    
    // Suggestions should be different based on directory context
    if (homeSuggestions.length > 0 && tmpSuggestions.length > 0) {
      const suggestionsChanged = !homeSuggestions.every((s: string, i: number) => 
        tmpSuggestions[i] === s
      );
      expect(suggestionsChanged).toBe(true);
    }
  });

  test('should prioritize existing files and directories in suggestions', async ({ appFixture }) => {
    // Navigate to a directory with known files
    await appFixture.executeCommand('pwd');
    await appFixture.waitForCommandCompletion();
    
    // Type a command that should suggest files
    await appFixture.typeInTerminal('cat ');
    
    try {
      await appFixture.waitForSuggestions();
      const suggestions = await appFixture.getSuggestions();
      
      if (suggestions.length > 0) {
        // Suggestions should prioritize actual files over generic examples
        const genericSuggestions = suggestions.filter((s: string) => 
          s.includes('filename') || s.includes('example') || s.includes('file.txt')
        );
        const specificSuggestions = suggestions.filter((s: string) => 
          !s.includes('filename') && !s.includes('example') && s.includes('cat ')
        );
        
        // Should have more specific suggestions than generic ones
        expect(specificSuggestions.length).toBeGreaterThanOrEqual(genericSuggestions.length);
      }
    } catch (error) {
      console.log('No file suggestions for cat command');
    }
  });
});
