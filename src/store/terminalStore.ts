import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface CommandExecution {
  id: string;
  command: string;
  output: string;
  exit_code?: number;
  duration_ms: number;
  timestamp: string;
}

interface TerminalSession {
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
  setActiveSession: (sessionId: string) => void;
  executeCommand: (command: string) => Promise<void>;
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

  setCurrentInput: (input: string) => {
    set({ currentInput: input });
  },

  getHistory: () => {
    return get().commandHistory;
  },
}));
