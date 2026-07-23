import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export interface CommandExecution {
  id: string;
  session_id: string;
  command: string;
  output: string;
  exit_code?: number;
  duration_ms: number;
  timestamp: string;
  working_directory: string;
}

export interface TerminalSession {
  id: string;
  title: string;
  working_directory: string;
  is_active: boolean;
  created_at: string;
}

interface TerminalState {
  sessions: TerminalSession[];
  activeSession: string | null;
  commandHistory: CommandExecution[];
  currentInput: string;
  isExecuting: boolean;
  isInitialized: boolean;
  isInitializing: boolean;

  // Actions
  createSession: (title?: string) => Promise<string | null>;
  restartSession: (sessionId: string) => Promise<string | null>;
  closeSession: (sessionId: string) => Promise<void>;
  updateSessionTitle: (sessionId: string, title: string) => Promise<void>;
  setActiveSession: (sessionId: string) => void;
  updateSessionWorkingDirectory: (sessionId: string, workingDirectory: string) => void;
  recordCommandExecution: (execution: CommandExecution) => void;
  executeCommand: (command: string) => Promise<void>;
  clearHistory: () => Promise<void>;
  setCurrentInput: (input: string) => void;
  getHistory: () => CommandExecution[];
  initializeDefaultSessions: () => Promise<void>;
  loadCommandHistory: () => Promise<void>;
  selectWorkspace: () => Promise<string | null>;
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
  sessions: [],
  activeSession: null,
  commandHistory: [],
  currentInput: '',
  isExecuting: false,
  isInitialized: false,
  isInitializing: false,

  createSession: async (title?: string) => {
    try {
      const sessionId = await invoke<string>('create_terminal', { title, workingDirectory: null });
      const backendSessions = await invoke<TerminalSession[]>('get_all_sessions');
      const newSession = backendSessions.find(session => session.id === sessionId) ?? {
        id: sessionId,
        title: title || `Terminal ${sessionId.slice(0, 8)}`,
        working_directory: '~',
        is_active: true,
        created_at: new Date().toISOString(),
      };

      set(state => ({
        sessions: [...state.sessions, newSession],
        activeSession: sessionId,
      }));

      return sessionId;
    } catch (error) {
      console.error('Failed to create terminal session:', error);
      return null;
    }
  },

  restartSession: async (sessionId: string) => {
    const previous = get().sessions.find(session => session.id === sessionId);
    if (!previous) return null;

    try {
      // Native replacement is transactional: the new PTY is validated and
      // started before the dead one is retired, including at the tab limit.
      const replacement = await invoke<TerminalSession>('restart_terminal_session', { sessionId });
      set(state => ({
        sessions: state.sessions.map(session =>
          session.id === sessionId ? replacement : session
        ),
        activeSession: state.activeSession === sessionId ? replacement.id : state.activeSession,
      }));
      return replacement.id;
    } catch (error) {
      console.error('Failed to restart terminal session:', error);
      return null;
    }
  },

  selectWorkspace: async () => {
    const { activeSession } = get();
    if (!activeSession) return null;

    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Choose a terminal workspace',
      });

      if (typeof selectedPath !== 'string') return null;

      const workingDirectory = await invoke<string>('change_directory', {
        sessionId: activeSession,
        newPath: selectedPath,
      });

      set(state => ({
        sessions: state.sessions.map(session =>
          session.id === activeSession
            ? { ...session, working_directory: workingDirectory }
            : session
        ),
      }));
      return workingDirectory;
    } catch (error) {
      console.error('Failed to select workspace:', error);
      throw error;
    }
  },

  closeSession: async (sessionId: string) => {
    try {
      await invoke('close_terminal_session', { sessionId });

      set(state => {
        const updatedSessions = state.sessions.filter(session => session.id !== sessionId);
        let newActiveSession = state.activeSession;

        // If we're closing the active session, switch to another one
        if (state.activeSession === sessionId) {
          newActiveSession = updatedSessions.length > 0 ? updatedSessions[0].id : null;
        }

        return {
          sessions: updatedSessions,
          activeSession: newActiveSession,
        };
      });

    } catch (error) {
      console.error('Failed to close terminal session:', error);
    }
  },

  updateSessionTitle: async (sessionId: string, title: string) => {
    try {
      await invoke('update_session_title', { sessionId, title });

      set(state => ({
        sessions: state.sessions.map(session =>
          session.id === sessionId ? { ...session, title } : session
        ),
      }));

    } catch (error) {
      console.error('Failed to update session title:', error);
    }
  },

  setActiveSession: (sessionId: string) => {
    set({ activeSession: sessionId });
  },

  updateSessionWorkingDirectory: (sessionId: string, workingDirectory: string) => {
    set(state => ({
      sessions: state.sessions.map(session =>
        session.id === sessionId
          ? { ...session, working_directory: workingDirectory }
          : session
      ),
    }));
  },

  recordCommandExecution: (execution: CommandExecution) => {
    set(state => {
      if (state.commandHistory.some(item => item.id === execution.id)) return state;
      return {
        commandHistory: [...state.commandHistory, execution].slice(-1_000),
      };
    });
  },

  executeCommand: async (command: string) => {
    const { activeSession } = get();
    if (!activeSession || !command.trim()) return;

    set({ isExecuting: true });

    try {
      const execution = await invoke<CommandExecution>('execute_command', {
        sessionId: activeSession,
        command: command.trim(),
      });

      set(state => ({
        commandHistory: [...state.commandHistory, execution],
        currentInput: '',
        isExecuting: false,
      }));
    } catch (error) {
      console.error('Failed to execute command:', error);
      set({ isExecuting: false });
    }
  },

  clearHistory: async () => {
    await invoke('clear_command_history');
    set({ commandHistory: [] });
  },

  setCurrentInput: (input: string) => {
    set({ currentInput: input });
  },

  getHistory: () => {
    return get().commandHistory;
  },

  initializeDefaultSessions: async () => {
    const { isInitialized, isInitializing } = get();

    if (isInitialized || isInitializing) {
      console.log('📝 Terminal store already initialized, skipping default session creation');
      return;
    }

    // Claim initialization synchronously before the first await. React's
    // development StrictMode intentionally replays effects; without this
    // guard both invocations could create a native PTY and duplicate the tab.
    set({ isInitializing: true });

    console.log('📝 Initializing terminal sessions...');

    try {
      // Older builds persisted session titles and workspace paths in WebView
      // localStorage. Remove that plaintext legacy state and start a single
      // fresh PTY. Searchable command history is stored separately by the
      // encrypted native history service.
      localStorage.removeItem('pH7Console_sessions');
      const sessionId = await get().createSession('Main Terminal');
      if (!sessionId) throw new Error('The native terminal session did not start');

      await get().loadCommandHistory();
      set({ isInitialized: true, isInitializing: false });
    } catch (error) {
      console.error('Failed to initialize sessions:', error);
      // Fallback: create at least one session
      if (get().sessions.length === 0) {
        await get().createSession('Main Terminal');
      }
      await get().loadCommandHistory();
      set({ isInitialized: true, isInitializing: false });
    }
  },

  loadCommandHistory: async () => {
    try {
      const commandHistory = await invoke<CommandExecution[]>('get_recent_command_history', {
        limit: 500,
      });
      set({ commandHistory });
    } catch (error) {
      // A history database problem must not prevent a shell from opening.
      console.error('Failed to load local command history:', error);
    }
  },

}));
