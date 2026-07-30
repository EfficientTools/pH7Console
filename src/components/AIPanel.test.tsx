import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAIStore } from '../store/aiStore';
import { TerminalSession, useTerminalStore } from '../store/terminalStore';
import { AIPanel } from './AIPanel';

vi.mock('@tauri-apps/api/core');

const session: TerminalSession = {
  id: 'review-session',
  title: 'Review terminal',
  working_directory: '/tmp/project',
  is_active: true,
  created_at: '2026-07-30T00:00:00Z',
};

describe('local command planning availability', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useTerminalStore.setState({
      activeSession: session.id,
      sessions: [session],
      commandHistory: [],
    });
    useAIStore.setState({
      isModelLoaded: false,
      isProcessing: false,
      modelError: 'The on-device model is still warming',
      suggestions: [],
      realLlmStatus: {
        available: false,
        backend: null,
        models: [],
        message: 'Deterministic local planning is ready',
      },
    });

    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'get_voice_input_status') {
        return {
          kind: 'status',
          available: false,
          microphoneAuthorization: 'not_determined',
          speechAuthorization: 'not_determined',
          message: 'Voice is unavailable in this test',
        };
      }
      if (command === 'create_command_plan') {
        return {
          command: 'git branch --show-current',
          confidence: 0.9,
          explanation: 'Shows the current branch.',
          source: 'deterministic',
          riskLevel: 'low',
          riskReasons: [],
          requiresConfirmation: false,
          requiresStrongConfirmation: false,
        };
      }
      if (command === 'get_local_llm_status') {
        return {
          available: false,
          backend: null,
          models: [],
          message: 'Deterministic local planning is ready',
        };
      }
      return undefined;
    });
  });

  it('keeps the planner interactive before the language model is ready', async () => {
    render(<AIPanel />);

    const input = screen.getByRole('textbox', { name: 'Describe a command to plan' });
    expect(input).toBeEnabled();
    fireEvent.change(input, { target: { value: 'show the current git branch' } });

    const submit = screen.getByRole('button', { name: 'Create Safe Command Plan' });
    expect(submit).toBeEnabled();
    fireEvent.click(submit);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_command_plan', {
        sessionId: session.id,
        input: 'show the current git branch',
      });
    });
    expect(await screen.findByText('git branch --show-current')).toBeVisible();
  });
});
