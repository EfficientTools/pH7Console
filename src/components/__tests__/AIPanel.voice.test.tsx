import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { AIPanel } from '../AIPanel';
import { useAIStore } from '../../store/aiStore';
import { useTerminalStore } from '../../store/terminalStore';
import type { VoiceInputEvent } from '../../voice/voiceInput';

const eventMock = vi.hoisted(() => ({
  listener: undefined as ((event: { payload: VoiceInputEvent }) => void) | undefined,
}));

vi.mock('@tauri-apps/api/core');
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (_name: string, listener: (event: { payload: VoiceInputEvent }) => void) => {
    eventMock.listener = listener;
    return vi.fn();
  }),
}));

describe('AIPanel local inputs', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventMock.listener = undefined;
    useAIStore.setState({
      isModelLoaded: true,
      isProcessing: false,
      modelError: null,
      suggestions: [],
      realLlmStatus: {
        available: true,
        backend: 'local',
        models: ['test'],
        message: 'On-device model ready',
      },
    });
    useTerminalStore.setState({ activeSession: 'voice-test', commandHistory: [] });

    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'get_voice_input_status') {
        return {
          kind: 'status',
          available: true,
          onDeviceAvailable: true,
          microphoneAuthorization: 'authorized',
          speechAuthorization: 'authorized',
          message: 'On-device voice input is ready.',
        };
      }
      if (command === 'create_command_plan') {
        return {
          command: 'find . -type f -size +100M -print',
          confidence: 0.9,
          explanation: 'Find large files.',
          source: 'local_llm',
          riskLevel: 'low',
          riskReasons: [],
          requiresConfirmation: false,
          requiresStrongConfirmation: false,
        };
      }
      return undefined;
    });
  });

  it('places speech in the editable planner without submitting or executing it', async () => {
    const user = userEvent.setup();
    render(<AIPanel />);

    await waitFor(() => expect(eventMock.listener).toBeDefined());
    act(() => {
      eventMock.listener?.({
        payload: {
          kind: 'final',
          transcript: 'show me every file larger than one hundred megabytes',
          isFinal: true,
          available: true,
          onDeviceAvailable: true,
          message: 'Voice draft is ready for review.',
        },
      });
    });

    expect(screen.getByLabelText('Describe a command to plan')).toHaveValue(
      'show me every file larger than one hundred megabytes',
    );
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith(
      'create_command_plan',
      expect.anything(),
    );
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith(
      'write_to_terminal',
      expect.anything(),
    );

    await user.click(screen.getByRole('button', { name: 'Create Safe Command Plan' }));
    await waitFor(() => expect(vi.mocked(invoke)).toHaveBeenCalledWith(
      'create_command_plan',
      expect.objectContaining({
        sessionId: 'voice-test',
        input: 'show me every file larger than one hundred megabytes',
      }),
    ));
  });

  it('cancels on-device generation and ignores a late result', async () => {
    const user = userEvent.setup();
    let resolvePlan: ((value: unknown) => void) | undefined;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'get_voice_input_status') {
        return {
          kind: 'status',
          available: true,
          onDeviceAvailable: true,
          microphoneAuthorization: 'authorized',
          speechAuthorization: 'authorized',
          message: 'On-device voice input is ready.',
        };
      }
      if (command === 'create_command_plan') {
        return new Promise(resolve => {
          resolvePlan = resolve;
        });
      }
      if (command === 'cancel_ai_generation') return true;
      return undefined;
    });

    render(<AIPanel />);
    await user.type(screen.getByLabelText('Describe a command to plan'), 'find large files');
    await user.click(screen.getByRole('button', { name: 'Create Safe Command Plan' }));
    await user.click(await screen.findByRole('button', { name: 'Stop local AI generation' }));

    expect(vi.mocked(invoke)).toHaveBeenCalledWith('cancel_ai_generation');
    act(() => resolvePlan?.({
      command: 'find . -type f -size +100M -print',
      confidence: 0.9,
      explanation: 'Find large files.',
      source: 'local_llm',
      riskLevel: 'low',
      riskReasons: [],
      requiresConfirmation: false,
      requiresStrongConfirmation: false,
    }));

    await waitFor(() => {
      expect(screen.queryByText('find . -type f -size +100M -print')).not.toBeInTheDocument();
    });
  });
});
