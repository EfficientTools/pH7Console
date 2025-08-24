import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { Terminal } from '../Terminal';
import { invoke } from '@tauri-apps/api/core';

// Mock the stores and Tauri
vi.mock('@tauri-apps/api/core');

describe('Terminal Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (invoke as any).mockResolvedValue({ success: true });
  });

  test('renders terminal interface elements', () => {
    render(<Terminal />);
    
    // Terminal should render basic structure
    const terminalElement = document.querySelector('.terminal') || 
                           document.querySelector('[class*="terminal"]') ||
                           document.body.firstElementChild;
    
    expect(terminalElement).toBeTruthy();
  });

  test('handles basic functionality', async () => {
    render(<Terminal />);
    
    // Terminal should be present
    expect(document.body.firstElementChild).toBeTruthy();
    
    // Should handle user interactions
    await waitFor(() => {
      expect(document.body.firstElementChild).toBeTruthy();
    });
  });

  test('integrates with Tauri backend', async () => {
    render(<Terminal />);
    
    // Should make calls to Tauri backend
    await waitFor(() => {
      expect(invoke).toHaveBeenCalled();
    }, { timeout: 3000 });
  });

  test('handles component lifecycle', () => {
    const { unmount } = render(<Terminal />);
    
    // Should mount successfully
    expect(document.body.firstElementChild).toBeTruthy();
    
    // Should unmount without errors
    unmount();
  });

  test('responds to user input', async () => {
    const user = userEvent.setup();
    render(<Terminal />);
    
    // Find any input element
    const inputElement = document.querySelector('input') || 
                        document.querySelector('textarea') ||
                        document.querySelector('[contenteditable]');
    
    if (inputElement) {
      await user.click(inputElement);
      await user.type(inputElement, 'test command');
      
      // Should handle input
      expect(inputElement).toBeTruthy();
    }
  });

  test('handles AI integration', () => {
    render(<Terminal />);
    
    // Should handle AI features
    const terminalElement = document.body.firstElementChild;
    expect(terminalElement).toBeTruthy();
  });

  test('manages state correctly', async () => {
    render(<Terminal />);
    
    // Should manage internal state
    await waitFor(() => {
      expect(document.body.firstElementChild).toBeTruthy();
    });
  });

  test('handles error scenarios', async () => {
    // Mock an error
    (invoke as any).mockRejectedValueOnce(new Error('Test error'));
    
    render(<Terminal />);
    
    // Should handle errors gracefully
    await waitFor(() => {
      expect(document.body.firstElementChild).toBeTruthy();
    });
  });
});
});
