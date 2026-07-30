import { afterAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { TerminalSession, useTerminalStore } from './terminalStore';

vi.mock('@tauri-apps/api/core');
const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);

const previous: TerminalSession = {
  id: 'previous-session',
  title: 'Development',
  working_directory: '/tmp/project',
  is_active: true,
  created_at: '2026-07-21T00:00:00Z',
};

describe('terminal session recovery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useTerminalStore.setState({
      activeSession: previous.id,
      sessions: [previous],
      terminalError: null,
    });
  });

  it('atomically swaps a dead shell for its native replacement', async () => {
    const replacement: TerminalSession = {
      ...previous,
      id: 'replacement-session',
      created_at: '2026-07-21T00:01:00Z',
    };
    vi.mocked(invoke).mockResolvedValueOnce(replacement);

    await expect(useTerminalStore.getState().restartSession(previous.id)).resolves.toBe(
      replacement.id,
    );

    expect(invoke).toHaveBeenCalledWith('restart_terminal_session', {
      sessionId: previous.id,
    });
    expect(useTerminalStore.getState().sessions).toEqual([replacement]);
    expect(useTerminalStore.getState().activeSession).toBe(replacement.id);
  });

  it('keeps the existing tab when native replacement fails', async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error('shell unavailable'));

    await expect(useTerminalStore.getState().restartSession(previous.id)).resolves.toBeNull();

    expect(useTerminalStore.getState().sessions).toEqual([previous]);
    expect(useTerminalStore.getState().activeSession).toBe(previous.id);
    expect(useTerminalStore.getState().terminalError).toContain('shell unavailable');
  });

  it('makes a failed first shell actionable without hiding the terminal surface', async () => {
    useTerminalStore.setState({
      activeSession: null,
      sessions: [],
      terminalError: null,
    });
    vi.mocked(invoke).mockRejectedValueOnce(new Error('sandbox denied the PTY'));

    await expect(useTerminalStore.getState().createSession('Main Terminal')).resolves.toBeNull();

    expect(useTerminalStore.getState().sessions).toEqual([]);
    expect(useTerminalStore.getState().terminalError).toBe(
      'The shell could not start. sandbox denied the PTY',
    );
  });
});

afterAll(() => consoleError.mockRestore());
