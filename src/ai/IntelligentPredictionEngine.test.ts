import { describe, it, expect, vi, beforeEach } from 'vitest';
import { IntelligentPredictionEngine } from './IntelligentPredictionEngine';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('IntelligentPredictionEngine', () => {
  let engine: IntelligentPredictionEngine;
  const mockInvoke = vi.mocked(invoke);
  
  const mockContext = {
    workingDirectory: '/test/directory',
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
    frequentDirectories: ['/home/user', '/projects'],
    availableCommands: ['ls', 'git', 'npm'],
    currentDirectoryContents: ['package.json', 'src', 'README.md']
  };

  beforeEach(() => {
    engine = new IntelligentPredictionEngine();
    vi.clearAllMocks();
  });

  describe('Command Validation', () => {
    it('should validate existing commands', async () => {
      // Mock successful command validation
      mockInvoke.mockResolvedValueOnce('/usr/bin/ls');
      
      const isValid = await (engine as any).validateCommand('ls -la');
      expect(isValid).toBe(true);
      expect(mockInvoke).toHaveBeenCalledWith('execute_simple_command', {
        command: 'which ls 2>/dev/null || echo "not found"',
        directory: expect.any(String)
      });
    });

    it('should invalidate non-existent commands', async () => {
      // Mock failed command validation
      mockInvoke.mockResolvedValueOnce('not found');
      
      const isValid = await (engine as any).validateCommand('invalidcommand');
      expect(isValid).toBe(false);
    });

    it('should cache command validation results', async () => {
      // First call
      mockInvoke.mockResolvedValueOnce('/usr/bin/git');
      await (engine as any).validateCommand('git status');
      
      // Second call should use cache
      await (engine as any).validateCommand('git log');
      
      // Should only call invoke once for the base command 'git'
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });
  });

  describe('Path Validation', () => {
    it('should validate existing paths', async () => {
      // Set up context first
      (engine as any).contextCache = mockContext;
      
      // Mock the invoke function to simulate existing path
      mockInvoke.mockResolvedValueOnce('exists');
      
      const correctedPath = await (engine as any).validateAndCorrectPath('./src');
      expect(correctedPath).toBe('./src');
      expect(mockInvoke).toHaveBeenCalledWith('execute_simple_command', {
        command: 'test -e "./src" && echo "exists" || echo "not found"',
        directory: mockContext.workingDirectory
      });
    });

    it('should return empty string for non-existent paths', async () => {
      // Mock failed path validation
      mockInvoke.mockResolvedValueOnce('not found');
      
      const correctedPath = await (engine as any).validateAndCorrectPath('./nonexistent');
      expect(correctedPath).toBe('');
    });

    it('should correct relative paths to full paths', async () => {
      // Mock context with frequent directories
      (engine as any).contextCache = {
        ...mockContext,
        workingDirectory: '/current',
        frequentDirectories: ['/home/user', '/projects']
      };

      // Mock path not found in current directory
      mockInvoke.mockResolvedValueOnce('not found');
      // Mock path found in frequent directory
      mockInvoke.mockResolvedValueOnce('exists');
      
      const correctedPath = await (engine as any).validateAndCorrectPath('documents');
      expect(correctedPath).toBe('/home/user/documents');
    });
  });

  describe('Intent Detection', () => {
    it('should detect file search intent', () => {
      const intents = (engine as any).detectIntents('find large files');
      expect(intents).toContain('file_search');
    });

    it('should detect development intent', () => {
      const intents = (engine as any).detectIntents('git status');
      expect(intents).toContain('development');
    });

    it('should detect system monitoring intent', () => {
      const intents = (engine as any).detectIntents('show cpu usage');
      expect(intents).toContain('system_monitor');
    });

    it('should fallback to general intent for unknown commands', () => {
      const intents = (engine as any).detectIntents('unknown command');
      expect(intents).toContain('general');
    });
  });

  describe('Suggestion Validation', () => {
    it('should filter out invalid suggestions', async () => {
      // Mock command validation - first invalid, second valid
      mockInvoke
        .mockResolvedValueOnce('not found') // invalidcommand
        .mockResolvedValueOnce('/usr/bin/ls'); // ls
      
      const suggestions = ['invalidcommand', 'ls -la'];
      const validSuggestions = await (engine as any).validateSuggestions(suggestions);
      
      expect(validSuggestions).toEqual(['ls -la']);
      expect(validSuggestions).not.toContain('invalidcommand');
    });

    it('should correct paths in suggestions', async () => {
      // Setup context
      (engine as any).contextCache = {
        workingDirectory: '/current',
        frequentDirectories: ['/home/user']
      };

      // Mock command validation (ls exists)
      mockInvoke.mockResolvedValueOnce('/bin/ls');
      // Mock path validation (file not in current dir)
      mockInvoke.mockResolvedValueOnce('not found');
      // Mock path found in frequent directory
      mockInvoke.mockResolvedValueOnce('exists');
      
      const suggestions = ['ls documents/file.txt'];
      const validSuggestions = await (engine as any).validateSuggestions(suggestions);
      
      expect(validSuggestions[0]).toContain('/home/user/documents/file.txt');
    });
  });

  describe('Context-Aware Suggestions', () => {
    it('should provide suggestions based on current directory files', async () => {
      // Set up context first
      (engine as any).contextCache = mockContext;
      
      // Mock current directory files
      mockInvoke.mockResolvedValueOnce('package.json\nsrc\nREADME.md');
      
      const files = await (engine as any).getCurrentDirectoryFiles();
      expect(files).toEqual(['package.json', 'src', 'README.md']);
    });

    it('should suggest project-specific commands based on files', async () => {
      // Setup context with project files
      (engine as any).contextCache = {
        ...mockContext,
        workingDirectory: '/project',
        projectType: 'node'
      };

      // Mock getCurrentDirectoryFiles to return files including package.json
      const mockGetCurrentDirectoryFiles = vi.spyOn(engine as any, 'getCurrentDirectoryFiles');
      mockGetCurrentDirectoryFiles.mockResolvedValueOnce(['package.json', 'src', 'node_modules']);
      
      // Mock validateSuggestions to return all suggestions as valid
      const mockValidateSuggestions = vi.spyOn(engine as any, 'validateSuggestions');
      mockValidateSuggestions.mockResolvedValueOnce([
        'npm install',
        'npm run dev', 
        'npm test',
        'npm run build'
      ]);
      
      const prediction = await (engine as any).generateDevelopmentPrediction('npm');
      
      expect(prediction.suggestions).toContain('npm install');
      expect(prediction.suggestions).toContain('npm run dev');
      
      // Cleanup
      mockGetCurrentDirectoryFiles.mockRestore();
      mockValidateSuggestions.mockRestore();
    });
  });

  describe('Performance and Caching', () => {
    it('should cache validated commands for performance', async () => {
      // First validation
      mockInvoke.mockResolvedValueOnce('/usr/bin/git');
      await (engine as any).validateCommand('git status');
      
      // Second validation of same base command
      await (engine as any).validateCommand('git log');
      
      // Should only call invoke once due to caching
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    it('should cache path validations', async () => {
      // Set up context first
      (engine as any).contextCache = mockContext;
      
      // First validation
      mockInvoke.mockResolvedValueOnce('exists');
      await (engine as any).validateAndCorrectPath('./src');
      
      // Second validation of same path
      await (engine as any).validateAndCorrectPath('./src');
      
      // Should only call invoke once due to caching
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });
  });
});
