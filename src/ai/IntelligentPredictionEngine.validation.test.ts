import { describe, it, expect, vi, beforeEach } from 'vitest';
import { IntelligentPredictionEngine } from './IntelligentPredictionEngine';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('IntelligentPredictionEngine - Validation Edge Cases', () => {
  let engine: IntelligentPredictionEngine;
  const mockInvoke = vi.mocked(invoke);
  
  const mockContext = {
    workingDirectory: '/Users/testuser',
    projectType: 'node',
    runningProcesses: ['node', 'npm'],
    systemResources: {
      cpu: 30,
      memory: 45,
      disk: 60
    },
    recentFiles: ['package.json', 'README.md'],
    gitStatus: {
      branch: 'main',
      hasChanges: false,
      ahead: 0,
      behind: 0
    },
    frequentDirectories: ['/Users/testuser/Projects', '/Users/testuser/DirectoryZ', '/Users/testuser/Downloads'],
    availableCommands: ['ls', 'git', 'npm'],
    currentDirectoryContents: ['package.json', 'src', 'README.md']
  };

  beforeEach(() => {
    engine = new IntelligentPredictionEngine();
    vi.clearAllMocks();
  });

  describe('Frequent Directory Validation', () => {
    it('should remove non-existent directories from frequent directories', async () => {
      // Set up context with some non-existent directories
      (engine as any).contextCache = mockContext;
      
      // Mock validate_frequent_directories to return only existing directories
      mockInvoke.mockResolvedValueOnce([
        '/Users/testuser/Projects',
        '/Users/testuser/Downloads'
        // DirectoryZ is missing because it was deleted
      ]);
      
      const validDirs = await (engine as any).validateFrequentDirectories();
      
      expect(mockInvoke).toHaveBeenCalledWith('validate_frequent_directories', {
        frequentDirs: ['/Users/testuser/Projects', '/Users/testuser/DirectoryZ', '/Users/testuser/Downloads'],
        currentWorkingDir: mockContext.workingDirectory
      });
      
      expect(validDirs).toEqual([
        '/Users/testuser/Projects',
        '/Users/testuser/Downloads'
      ]);
      
      // Context should be updated with cleaned directories
      expect((engine as any).contextCache.frequentDirectories).toEqual([
        '/Users/testuser/Projects',
        '/Users/testuser/Downloads'
      ]);
    });

    it('should handle empty frequent directories gracefully', async () => {
      const contextWithoutFrequentDirs = { ...mockContext, frequentDirectories: undefined };
      (engine as any).contextCache = contextWithoutFrequentDirs;
      
      const validDirs = await (engine as any).validateFrequentDirectories();
      
      expect(validDirs).toEqual([]);
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });

  describe('Path Correction and Auto-fixing', () => {
    it('should correct relative paths to absolute paths regardless of current directory', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock validate_and_correct_path to return corrected absolute path
      mockInvoke.mockResolvedValueOnce('/Users/testuser/Projects/myproject');
      
      const correctedPath = await (engine as any).validateAndCorrectPath('myproject');
      
      expect(mockInvoke).toHaveBeenCalledWith('validate_and_correct_path', {
        path: 'myproject',
        currentWorkingDir: '/Users/testuser',
        frequentDirectories: mockContext.frequentDirectories
      });
      
      expect(correctedPath).toBe('/Users/testuser/Projects/myproject');
    });

    it('should find directories in common locations when not in frequent directories', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock find_path_in_common_locations
      mockInvoke.mockResolvedValueOnce('/Users/testuser/Desktop/SomeFolder');
      
      const foundPath = await (engine as any).findPathInCommonLocations('SomeFolder');
      
      expect(mockInvoke).toHaveBeenCalledWith('find_path_in_common_locations', {
        targetName: 'SomeFolder',
        currentWorkingDir: '/Users/testuser'
      });
      
      expect(foundPath).toBe('/Users/testuser/Desktop/SomeFolder');
    });

    it('should handle tilde (~) expansion correctly', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock that the path exists after tilde expansion
      mockInvoke.mockResolvedValueOnce('/Users/testuser/Documents/ImportantFolder');
      
      const correctedPath = await (engine as any).validateAndCorrectPath('~/Documents/ImportantFolder');
      
      expect(correctedPath).toBe('/Users/testuser/Documents/ImportantFolder');
    });

    it('should return empty string for completely invalid paths', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock that no valid path is found
      mockInvoke.mockResolvedValueOnce(null);
      
      const correctedPath = await (engine as any).validateAndCorrectPath('completely/invalid/path');
      
      expect(correctedPath).toBe('');
    });
  });

  describe('Command Suggestion Validation', () => {
    it('should validate all command suggestions before returning them', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock getCurrentDirectoryFiles
      const mockGetCurrentDirectoryFiles = vi.spyOn(engine as any, 'getCurrentDirectoryFiles');
      mockGetCurrentDirectoryFiles.mockResolvedValueOnce(['file1.txt', 'file2.js']);
      
      // Mock validateFrequentDirectories to return valid directories
      mockInvoke.mockResolvedValueOnce(['/Users/testuser/Projects', '/Users/testuser/Downloads']);
      
      // Mock validateSuggestions to filter out invalid commands
      const mockValidateSuggestions = vi.spyOn(engine as any, 'validateSuggestions');
      mockValidateSuggestions.mockResolvedValueOnce([
        'cd "/Users/testuser/Projects"  # Projects',
        'ls -la /Users/testuser/Downloads'
      ]);
      
      const prediction = await (engine as any).generateFileManagementPrediction('cd to my projects');
      
      expect(prediction.suggestions).toContain('cd "/Users/testuser/Projects"  # Projects');
      expect(mockValidateSuggestions).toHaveBeenCalled();
      
      // Cleanup
      mockGetCurrentDirectoryFiles.mockRestore();
      mockValidateSuggestions.mockRestore();
    });

    it('should handle navigation to deleted frequent directories gracefully', async () => {
      // Context with a deleted directory in frequent list
      const contextWithDeletedDir = {
        ...mockContext,
        frequentDirectories: ['/Users/testuser/Projects', '/Users/testuser/DeletedDirectory']
      };
      (engine as any).contextCache = contextWithDeletedDir;
      
      // Mock getCurrentDirectoryFiles
      const mockGetCurrentDirectoryFiles = vi.spyOn(engine as any, 'getCurrentDirectoryFiles');
      mockGetCurrentDirectoryFiles.mockResolvedValueOnce(['file1.txt']);
      
      // Mock that validation removes the deleted directory
      mockInvoke.mockResolvedValueOnce(['/Users/testuser/Projects']);
      
      // Mock the path finding for any remaining suggestions
      const mockValidateSuggestions = vi.spyOn(engine as any, 'validateSuggestions');
      mockValidateSuggestions.mockResolvedValueOnce(['cd "/Users/testuser/Projects"  # Projects']);
      
      const prediction = await (engine as any).generateFileManagementPrediction('cd');
      
      // Should only suggest existing directories
      expect(prediction.suggestions).not.toContain('DeletedDirectory');
      expect(prediction.suggestions.some((s: string) => s.includes('/Users/testuser/Projects'))).toBe(true);
      
      // Cleanup
      mockGetCurrentDirectoryFiles.mockRestore();
      mockValidateSuggestions.mockRestore();
    });
  });

  describe('Context-aware Path Resolution', () => {
    it('should provide full paths when user is in different directory', async () => {
      // User is in home directory but frequently goes to a project directory
      const homeContext = {
        ...mockContext,
        workingDirectory: '/Users/testuser',
        frequentDirectories: ['/Users/testuser/Documents/Projects/MyApp']
      };
      (engine as any).contextCache = homeContext;
      
      // Mock validation that confirms the frequent directory exists
      mockInvoke.mockResolvedValueOnce(['/Users/testuser/Documents/Projects/MyApp']);
      
      // Mock finding the path
      mockInvoke.mockResolvedValueOnce('/Users/testuser/Documents/Projects/MyApp');
      
      // Mock validation of suggestions
      const mockValidateSuggestions = vi.spyOn(engine as any, 'validateSuggestions');
      mockValidateSuggestions.mockResolvedValueOnce([
        'cd "/Users/testuser/Documents/Projects/MyApp"  # MyApp'
      ]);
      
      const prediction = await (engine as any).generateFileManagementPrediction('cd MyApp');
      
      // Should suggest full absolute path, not relative
      expect(prediction.suggestions.some((s: string) => 
        s.includes('/Users/testuser/Documents/Projects/MyApp')
      )).toBe(true);
    });

    it('should handle multiple potential matches and prioritize existing ones', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock validation returning only existing directories
      mockInvoke
        .mockResolvedValueOnce(['/Users/testuser/Projects']) // validateFrequentDirectories
        .mockResolvedValueOnce('/Users/testuser/Desktop/test-project'); // findPathInCommonLocations
      
      const mockValidateSuggestions = vi.spyOn(engine as any, 'validateSuggestions');
      mockValidateSuggestions.mockResolvedValueOnce([
        'cd "/Users/testuser/Projects"  # Projects',
        'cd "/Users/testuser/Desktop/test-project"  # Found: test'
      ]);
      
      const prediction = await (engine as any).generateFileManagementPrediction('cd test');
      
      expect(prediction.suggestions.length).toBeGreaterThan(0);
      expect(prediction.suggestions.every((s: string) => s.includes('cd "/'))).toBe(true);
    });
  });

  describe('Error Handling and Resilience', () => {
    it('should handle Tauri command failures gracefully', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock Tauri command failure
      mockInvoke.mockRejectedValueOnce(new Error('Tauri command failed'));
      
      const correctedPath = await (engine as any).validateAndCorrectPath('some/path');
      
      expect(correctedPath).toBe('');
    });

    it('should continue working when frequent directory validation fails', async () => {
      (engine as any).contextCache = mockContext;
      
      // Mock validation failure
      mockInvoke.mockRejectedValueOnce(new Error('Validation failed'));
      
      const validDirs = await (engine as any).validateFrequentDirectories();
      
      // Should fallback to original frequent directories
      expect(validDirs).toEqual(mockContext.frequentDirectories || []);
    });

    it('should handle edge cases in path names', async () => {
      (engine as any).contextCache = mockContext;
      
      const edgeCasePaths = [
        'path with spaces',
        'path-with-dashes',
        'path_with_underscores',
        'path.with.dots',
        'UPPERCASE_PATH',
        'mixed-Case_Path.dir'
      ];
      
      for (const path of edgeCasePaths) {
        mockInvoke.mockResolvedValueOnce(`/full/path/to/${path}`);
        
        const result = await (engine as any).validateAndCorrectPath(path);
        
        expect(result).toBe(`/full/path/to/${path}`);
      }
    });
  });
});
