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
  
  // Actions
  createSession: (title?: string) => Promise<void>;
  closeSession: (sessionId: string) => Promise<void>;
  updateSessionTitle: (sessionId: string, title: string) => Promise<void>;
  setActiveSession: (sessionId: string) => void;
  executeCommand: (command: string) => Promise<void>;
  clearHistory: () => void;
  setCurrentInput: (input: string) => void;
  getHistory: () => CommandExecution[];
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
  sessions: [],
  activeSession: null,
  commandHistory: [],
  currentInput: '',
  isExecuting: false,

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
}));
