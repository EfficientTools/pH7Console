import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface AIResponse {
  text: string;
  confidence: number;
  reasoning?: string;
}

interface AISuggestion {
  id: string;
  type: 'command' | 'explanation' | 'fix' | 'completion' | 'optimization' | 'analysis' | 'error';
  content: string;
  confidence: number;
  timestamp: number;
}

interface AIState {
  isModelLoaded: boolean;
  suggestions: AISuggestion[];
  currentAnalysis: string | null;
  isProcessing: boolean;
  
  // Actions
  loadModel: () => Promise<void>;
  getSuggestions: (context: string, intent?: string) => Promise<void>;
  explainCommand: (command: string) => Promise<AIResponse>;
  fixError: (error: string, command: string, context: string) => Promise<AIResponse>;
  analyzeOutput: (output: string, command: string) => Promise<AIResponse>;
  getCompletions: (partialCommand: string, sessionId: string) => Promise<string[]>;
  translateNaturalLanguage: (text: string, context: string) => Promise<AIResponse>;
  addSuggestion: (suggestion: AISuggestion) => void;
  clearSuggestions: () => void;
  
  // Learning features
  updateFeedback: (command: string, feedback: number) => Promise<void>;
  getUserAnalytics: () => Promise<UserAnalytics | null>;
  
  // Agent mode
  createAgentTask: (description: string) => Promise<string>;
  getAgentTaskStatus: (taskId: string) => Promise<string | null>;
  getActiveAgentTasks: () => Promise<string[]>;
  cancelAgentTask: (taskId: string) => Promise<void>;
}

interface UserAnalytics {
  total_commands: number;
  success_rate: number;
  most_used_commands: [string, number][];
  learning_examples: number;
  patterns_learned: number;
}

export const useAIStore = create<AIState>((set, get) => ({
  isModelLoaded: false,
  suggestions: [],
  currentAnalysis: null,
  isProcessing: false,

  loadModel: async () => {
    set({ isProcessing: true });
    try {
      // Model loading is handled in Rust backend
      await new Promise(resolve => setTimeout(resolve, 1000)); // Simulate loading
      set({ isModelLoaded: true, isProcessing: false });
    } catch (error) {
      console.error('Failed to load AI model:', error);
      set({ isProcessing: false });
    }
  },

  getSuggestions: async (context: string, intent?: string) => {
    if (!get().isModelLoaded) return;
    
    set({ isProcessing: true });
    try {
      const response = await invoke<AIResponse>('ai_suggest_command', {
        context,
        intent,
      });

      const suggestion: AISuggestion = {
        id: Date.now().toString(),
        type: 'command',
        content: response.text,
        confidence: response.confidence,
        timestamp: Date.now(),
      };

      set(state => ({
        suggestions: [...state.suggestions, suggestion],
        isProcessing: false,
      }));
    } catch (error) {
      console.error('Failed to get AI suggestions:', error);
      set({ isProcessing: false });
    }
  },

  explainCommand: async (command: string) => {
    try {
      return await invoke<AIResponse>('ai_explain_command', { command });
    } catch (error) {
      console.error('Failed to explain command:', error);
      return { text: 'Unable to explain command', confidence: 0 };
    }
  },

  fixError: async (error: string, command: string, context: string) => {
    try {
      return await invoke<AIResponse>('ai_fix_error', {
        errorOutput: error,
        command,
        context,
      });
    } catch (error) {
      console.error('Failed to fix error:', error);
      return { text: 'Unable to suggest fix', confidence: 0 };
    }
  },

  analyzeOutput: async (output: string, command: string) => {
    try {
      return await invoke<AIResponse>('ai_analyze_output', { output, command });
    } catch (error) {
      console.error('Failed to analyze output:', error);
      return { text: 'Unable to analyze output', confidence: 0 };
    }
  },

  getCompletions: async (partialCommand: string, sessionId: string) => {
    try {
      return await invoke<string[]>('get_smart_completions', {
        partialCommand,
        sessionId,
      });
    } catch (error) {
      console.error('Failed to get completions:', error);
      return [];
    }
  },

  translateNaturalLanguage: async (text: string, context: string) => {
    if (!get().isModelLoaded) {
      return { text: 'AI model not loaded', confidence: 0 };
    }
    
    set({ isProcessing: true });
    try {
      const response = await invoke<AIResponse>('ai_translate_natural_language', {
        naturalLanguage: text,
        context,
      });
      
      // Add as a suggestion if it's a valid command
      if (response.text && !response.text.startsWith('#') && !response.text.includes('need more')) {
        const suggestion: AISuggestion = {
          id: Date.now().toString(),
          type: 'command',
          content: response.text,
          confidence: response.confidence,
          timestamp: Date.now(),
        };
        
        set(state => ({
          suggestions: [...state.suggestions, suggestion],
          isProcessing: false,
        }));
      } else {
        set({ isProcessing: false });
      }
      
      return response;
    } catch (error) {
      console.error('Failed to translate natural language:', error);
      set({ isProcessing: false });
      return { text: 'Unable to translate', confidence: 0 };
    }
  },

  addSuggestion: (suggestion: AISuggestion) => {
    set(state => ({
      suggestions: [...state.suggestions, suggestion]
    }));
  },

  clearSuggestions: () => {
    set({ suggestions: [] });
  },

  updateFeedback: async (command: string, feedback: number) => {
    try {
      await invoke('update_ai_feedback', { command, feedback });
    } catch (error) {
      console.error('Failed to update feedback:', error);
    }
  },

  getUserAnalytics: async () => {
    try {
      return await invoke<UserAnalytics | null>('get_user_analytics');
    } catch (error) {
      console.error('Failed to get analytics:', error);
      return null;
    }
  },

  createAgentTask: async (description: string) => {
    try {
      return await invoke<string>('create_agent_task', { description });
    } catch (error) {
      console.error('Failed to create agent task:', error);
      throw error;
    }
  },

  getAgentTaskStatus: async (taskId: string) => {
    try {
      return await invoke<string | null>('get_agent_task_status', { taskId });
    } catch (error) {
      console.error('Failed to get task status:', error);
      return null;
    }
  },

  getActiveAgentTasks: async () => {
    try {
      return await invoke<string[]>('get_active_agent_tasks');
    } catch (error) {
      console.error('Failed to get active tasks:', error);
      return [];
    }
  },

  cancelAgentTask: async (taskId: string) => {
    try {
      await invoke('cancel_agent_task', { taskId });
    } catch (error) {
      console.error('Failed to cancel task:', error);
      throw error;
    }
  },
}));
