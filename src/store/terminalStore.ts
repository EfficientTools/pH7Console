import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface CommandExecution {
  id: string;
  command: string;
  output: string;
  exit_code?: number;
  duration_ms: number;
  timestamp: string;
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

  // Actions
  createSession: (title?: string) => Promise<void>;
  closeSession: (sessionId: string) => Promise<void>;
  updateSessionTitle: (sessionId: string, title: string) => Promise<void>;
  setActiveSession: (sessionId: string) => void;
  executeCommand: (command: string) => Promise<void>;
  clearHistory: () => void;
  setCurrentInput: (input: string) => void;
  getHistory: () => CommandExecution[];
  initializeDefaultSessions: () => Promise<void>;
  loadPersistedSessions: () => Promise<void>;
  persistSessions: () => Promise<void>;
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
  sessions: [],
  activeSession: null,
  commandHistory: [],
  currentInput: '',
  isExecuting: false,
  isInitialized: false,

  createSession: async (title?: string) => {
    try {
      const sessionId = await invoke<string>('create_terminal', { title });
      const newSession: TerminalSession = {
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

      // Persist sessions after creating
      get().persistSessions();
    } catch (error) {
      console.error('Failed to create terminal session:', error);
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

      // Persist sessions after closing
      get().persistSessions();
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

      // Persist sessions after updating title
      get().persistSessions();
    } catch (error) {
      console.error('Failed to update session title:', error);
    }
  },

  setActiveSession: (sessionId: string) => {
    set({ activeSession: sessionId });
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

  clearHistory: () => {
    set({ commandHistory: [] });
  },

  setCurrentInput: (input: string) => {
    set({ currentInput: input });
  },

  getHistory: () => {
    return get().commandHistory;
  },

  initializeDefaultSessions: async () => {
    const { sessions, isInitialized } = get();

    if (isInitialized) {
      console.log('📝 Terminal store already initialized, skipping default session creation');
      return;
    }

    console.log('📝 Initializing terminal sessions...');

    try {
      // First try to load persisted sessions
      await get().loadPersistedSessions();

      const { sessions: loadedSessions } = get();

      // If we have no sessions, create 2 default sessions for better UX
      if (loadedSessions.length === 0) {
        console.log('📝 No persisted sessions found, creating 2 default sessions for better workflow');
        await get().createSession('Main Terminal');
        await get().createSession('Terminal 2');
      } else {
        console.log(`📝 Found ${loadedSessions.length} persisted sessions, restoring them`);
      }

      set({ isInitialized: true });
    } catch (error) {
      console.error('Failed to initialize sessions:', error);
      // Fallback: create at least one session
      if (sessions.length === 0) {
        await get().createSession('Main Terminal');
      }
      set({ isInitialized: true });
    }
  },

  loadPersistedSessions: async () => {
    try {
      const sessionsData = localStorage.getItem('pH7Console_sessions');
      if (sessionsData) {
        const persistedSessions = JSON.parse(sessionsData) as TerminalSession[];
        console.log(`📝 Loading ${persistedSessions.length} persisted sessions`);

        // Recreate sessions in backend
        const validSessions: TerminalSession[] = [];
        let activeSessionId: string | null = null;

        for (const session of persistedSessions) {
          try {
            const sessionId = await invoke<string>('create_terminal', {
              title: session.title
            });

            const newSession: TerminalSession = {
              ...session,
              id: sessionId, // Use new backend session ID
              is_active: true,
            };

            validSessions.push(newSession);

            // Set the first session as active by default
            if (!activeSessionId) {
              activeSessionId = sessionId;
            }
          } catch (error) {
            console.error(`Failed to recreate session ${session.title}:`, error);
          }
        }

        set({
          sessions: validSessions,
          activeSession: activeSessionId,
        });
      }
    } catch (error) {
      console.error('Failed to load persisted sessions:', error);
    }
  },

  persistSessions: async () => {
    try {
      const { sessions } = get();
      // Only persist session metadata (not the backend session IDs)
      const sessionMetadata = sessions.map(session => ({
        id: session.id, // We'll generate new IDs when recreating
        title: session.title,
        working_directory: session.working_directory,
        is_active: session.is_active,
        created_at: session.created_at,
      }));

      localStorage.setItem('pH7Console_sessions', JSON.stringify(sessionMetadata));
      console.log(`📝 Persisted ${sessions.length} sessions to localStorage`);
    } catch (error) {
      console.error('Failed to persist sessions:', error);
    }
  },
}));
