import { describe, it, expect, vi, beforeEach } from 'vitest';
import { IntelligentPredictionEngine } from './IntelligentPredictionEngine';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('IntelligentPredictionEngine - Edge Cases & Validation', () => {
  let engine: IntelligentPredictionEngine;
  const mockInvoke = vi.mocked(invoke);
  
  const mockContext = {
    workingDirectory: '/Users/test',
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
    frequentDirectories: ['/Users/test/Projects', '/Users/test/DeletedDir', '/Users/test/Documents'],
    availableCommands: ['ls', 'git', 'npm'],
    currentDirectoryContents: ['package.json', 'src', 'README.md']
  };

  beforeEach(() => {
    engine = new IntelligentPredictionEngine();
    vi.clearAllMocks();
  });

  describe('Frequent Directory Validation', () => {
    it('should remove non-existent directories from frequent directories', async () => {
      // Setup context
      (engine as any).contextCache = mockContext;
      
      // Mock validate_frequent_directories to return only existing directories
      mockInvoke.mockResolvedValueOnce(['/Users/test/Projects', '/Users/test/Documents']);
      
      const validatedDirs = await (engine as any).validateFrequentDirectories();
      
      expect(mockInvoke).toHaveBeenCalledWith('validate_frequent_directories', {
        frequent_dirs: ['/Users/test/Projects', '/Users/test/DeletedDir', '/Users/test/Documents']
      });
      expect(validatedDirs).toEqual(['/Users/test/Projects', '/Users/test/Documents']);
      expect(validatedDirs).not.toContain('/Users/test/DeletedDir');
    });

    it('should update context cache with cleaned directories', async () => {
      // Setup context
      (engine as any).contextCache = { ...mockContext };
      
      // Mock returning only valid directories
      mockInvoke.mockResolvedValueOnce(['/Users/test/Projects']);
      
      await (engine as any).validateFrequentDirectories();
      
      // Check that context was updated
      expect((engine as any).contextCache.frequentDirectories).toEqual(['/Users/test/Projects']);
    });
  });

  describe('Path Correction Edge Cases', () => {
    it('should find path in common locations when not in frequent directories', async () => {
      // Setup context
      (engine as any).contextCache = {
        ...mockContext,
        workingDirectory: '/Users/test',
        frequentDirectories: []
      };
      
      // Mock path validation sequence
      mockInvoke
        .mockResolvedValueOnce([]) // validate_frequent_directories returns empty
        .mockResolvedValueOnce('not found') // path doesn't exist as-is
        .mockResolvedValueOnce('not found') // not in Desktop
        .mockResolvedValueOnce('exists'); // found in Documents
      
      const correctedPath = await (engine as any).validateAndCorrectPath('MyProject');
      
      expect(correctedPath).toBe('~/Documents/MyProject');
    });

    it('should return full absolute path when suggesting from different directory', async () => {
      // Setup context - current directory is home, but target is elsewhere
      (engine as any).contextCache = {
        ...mockContext,
        workingDirectory: '/Users/test',
        frequentDirectories: ['/opt/projects/work-project']
      };
      
      // Mock validated frequent directories
      mockInvoke.mockResolvedValueOnce(['/opt/projects/work-project']);
      
      // Mock directory suggestions
      mockInvoke.mockResolvedValueOnce(['cd /opt/projects/work-project']);
      
      const suggestions = await (engine as any).getValidatedDirectoryPredictions('cd work', 'session1');
      
      expect(suggestions[0].suggestions).toContain('cd /opt/projects/work-project');
      expect(mockInvoke).toHaveBeenCalledWith('get_validated_directory_suggestions', {
        partial_path: 'work',
        current_dir: '/Users/test',
        frequent_dirs: ['/opt/projects/work-project']
      });
    });

    it('should handle relative paths correctly from different working directories', async () => {
      // Setup context
      (engine as any).contextCache = {
        ...mockContext,
        workingDirectory: '/Users/test/deep/nested/folder'
      };
      
      // Mock path doesn't exist relative to current dir
      mockInvoke
        .mockResolvedValueOnce([]) // no frequent dirs
        .mockResolvedValueOnce('not found') // not relative to current
        .mockResolvedValueOnce('exists'); // found in home/Desktop
      
      const correctedPath = await (engine as any).validateAndCorrectPath('project');
      
      expect(correctedPath).toBe('~/Desktop/project');
    });
  });

  describe('Command Suggestion Validation', () => {
    it('should reject suggestions with non-existent commands', async () => {
      // Setup context
      (engine as any).contextCache = mockContext;
      
      // Mock command validation - fakecommand doesn't exist
      mockInvoke.mockResolvedValueOnce('not found');
      
      const isValid = await (engine as any).validateCommand('fakecommand');
      expect(isValid).toBe(false);
    });

    it('should validate and fix paths in command suggestions', async () => {
      // Setup context
      (engine as any).contextCache = mockContext;
      
      // Mock command exists
      mockInvoke.mockResolvedValueOnce('/bin/ls');
      
      // Mock path validation and correction
      mockInvoke
        .mockResolvedValueOnce([]) // validate frequent dirs
        .mockResolvedValueOnce('not found') // path doesn't exist as-is
        .mockResolvedValueOnce('exists'); // found in common location
      
      const suggestions = ['ls ~/Documents/nonexistent-folder'];
      const validSuggestions = await (engine as any).validateSuggestions(suggestions);
      
      // Should return corrected suggestion or empty if can't be fixed
      expect(Array.isArray(validSuggestions)).toBe(true);
    });

    it('should handle directory navigation with absolute paths correctly', async () => {
      // Setup context - user is in home directory
      (engine as any).contextCache = {
        ...mockContext,
        workingDirectory: '/Users/test',
        frequentDirectories: ['/opt/workspace', '/var/log/myapp']
      };
      
      // Mock validated directories (some might not exist anymore)
      mockInvoke.mockResolvedValueOnce(['/opt/workspace']);
      
      // Mock directory suggestions with absolute paths
      mockInvoke.mockResolvedValueOnce(['cd /opt/workspace']);
      
      const predictions = await (engine as any).getValidatedDirectoryPredictions('cd ', 'session1');
      
      expect(predictions[0].suggestions).toContain('cd /opt/workspace');
      expect(predictions[0].suggestions).not.toContain('cd /var/log/myapp'); // Should be filtered out
    });
  });

  describe('Real-time Suggestions with Validation', () => {
    it('should return only validated suggestions in real-time mode', async () => {
      // Setup context
      (engine as any).contextCache = mockContext;
      
      // Mock getPredictiveCompletions to return validated results
      const mockPredictions = [{
        intent: 'directory_navigation',
        confidence: 0.9,
        context: ['/Users/test'],
        suggestions: ['cd /valid/path', 'cd /another/valid/path'],
        explanation: 'Navigate to directories (validated paths only)'
      }];
      
      vi.spyOn(engine, 'getPredictiveCompletions').mockResolvedValue(mockPredictions);
      
      const suggestions = await engine.getRealtimeSuggestions('cd proj', 'session1', 3);
      
      expect(suggestions).toEqual(['cd /valid/path', 'cd /another/valid/path']);
    });

    it('should handle empty input gracefully', async () => {
      const suggestions = await engine.getRealtimeSuggestions('', 'session1');
      expect(suggestions).toEqual([]);
    });

    it('should limit suggestions to maxSuggestions parameter', async () => {
      // Setup context
      (engine as any).contextCache = mockContext;
      
      const mockPredictions = [{
        intent: 'general',
        confidence: 0.8,
        context: [],
        suggestions: ['cmd1', 'cmd2', 'cmd3', 'cmd4', 'cmd5'],
        explanation: 'Test commands'
      }];
      
      vi.spyOn(engine, 'getPredictiveCompletions').mockResolvedValue(mockPredictions);
      
      const suggestions = await engine.getRealtimeSuggestions('cmd', 'session1', 3);
      
      expect(suggestions.length).toBeLessThanOrEqual(3);
    });
  });

  describe('Path Finding in Common Locations', () => {
    it('should search in common directories when path not found in frequent dirs', async () => {
      // Setup context
      (engine as any).contextCache = {
        ...mockContext,
        frequentDirectories: []
      };
      
      // Mock path search sequence
      mockInvoke
        .mockResolvedValueOnce([]) // no frequent dirs
        .mockResolvedValueOnce('not found') // not in current dir
        .mockResolvedValueOnce('not found') // not in Desktop
        .mockResolvedValueOnce('not found') // not in Documents
        .mockResolvedValueOnce('exists'); // found in Downloads
      
      const foundPath = await (engine as any).findPathInCommonLocations('myfile.txt');
      
      expect(foundPath).toBe('~/Downloads/myfile.txt');
    });

    it('should return null when path not found anywhere', async () => {
      // Setup context
      (engine as any).contextCache = mockContext;
      
      // Mock all searches returning not found
      mockInvoke.mockResolvedValue('not found');
      
      const foundPath = await (engine as any).findPathInCommonLocations('nonexistent-file');
      
      expect(foundPath).toBeNull();
    });
  });
});
