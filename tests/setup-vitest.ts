import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock Tauri API
(globalThis as any).window = Object.create(window);
(globalThis as any).__TAURI__ = {
  invoke: vi.fn(),
  event: {
    listen: vi.fn(),
    emit: vi.fn(),
  }
};

// Mock the Tauri invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue('mocked response'),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
  emit: vi.fn(),
}));

// Mock clipboard API for tests
Object.assign(navigator, {
  clipboard: {
    writeText: vi.fn().mockResolvedValue(undefined),
    readText: vi.fn().mockResolvedValue(''),
  },
});

// Mock ResizeObserver
(globalThis as any).ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}));

// Mock scrollIntoView
Element.prototype.scrollIntoView = vi.fn();

// Global test configuration
export const TEST_CONFIG = {
  timeout: 30000,
  retries: 2,
  
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
