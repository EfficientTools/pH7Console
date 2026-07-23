import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface AIResponse {
  text: string;
  confidence: number;
  reasoning?: string;
  source: string;
}

export interface AISuggestion {
  id: string;
  type: 'command' | 'explanation' | 'fix' | 'completion' | 'optimization' | 'analysis' | 'error';
  content: string;
  confidence: number;
  timestamp: number;
  feedback?: 'positive' | 'negative' | null; // Track user feedback
  riskLevel?: 'low' | 'medium' | 'high' | 'critical';
  riskReasons?: string[];
  requiresConfirmation?: boolean;
  requiresStrongConfirmation?: boolean;
  source?: string;
}

interface AIState {
  isModelLoaded: boolean;
  realLlmStatus: LocalLlmStatus;
  modelError: string | null;
  suggestions: AISuggestion[];
  currentAnalysis: string | null;
  isProcessing: boolean;

  // Actions
  loadModel: () => Promise<void>;
  refreshLlmStatus: () => Promise<LocalLlmStatus>;
  getSuggestions: (context: string, intent?: string) => Promise<void>;
  explainCommand: (command: string) => Promise<AIResponse>;
  fixError: (error: string, command: string, context: string) => Promise<AIResponse>;
  analyzeOutput: (output: string, command: string) => Promise<AIResponse>;
  getCompletions: (partialCommand: string, sessionId: string) => Promise<string[]>;
  translateNaturalLanguage: (text: string, context: string) => Promise<AIResponse>;
  addSuggestion: (suggestion: AISuggestion) => void;
  clearSuggestions: () => void;

  // Learning features
  updateFeedback: (suggestionId: string, command: string, feedback: number) => Promise<void>;
  getUserAnalytics: () => Promise<UserAnalytics | null>;

}

interface UserAnalytics {
  total_commands: number;
  success_rate: number;
  most_used_commands: [string, number][];
  learning_examples: number;
  patterns_learned: number;
}

export interface LocalLlmStatus {
  available: boolean;
  backend: string | null;
  models: string[];
  message: string;
}

const MAX_SUGGESTIONS = 100;
let llmStatusPollGeneration = 0;

const appendSuggestion = (suggestions: AISuggestion[], suggestion: AISuggestion) =>
  [...suggestions, suggestion].slice(-MAX_SUGGESTIONS);

export const useAIStore = create<AIState>((set, get) => ({
  isModelLoaded: false,
  realLlmStatus: {
    available: false,
    backend: null,
    models: [],
    message: 'Checking for a verified local LLM runtime',
  },
  modelError: null,
  suggestions: [],
  currentAnalysis: null,
  isProcessing: false,

  loadModel: async () => {
    set({ isProcessing: true, modelError: null });
    try {
      await invoke<string>('initialize_ml_system');
      const realLlmStatus = await get().refreshLlmStatus();
      set({ isModelLoaded: true, realLlmStatus, isProcessing: false, modelError: null });
    } catch (error) {
      console.error('Failed to load AI model:', error);
      set({
        isModelLoaded: false,
        isProcessing: false,
        modelError: error instanceof Error ? error.message : 'Local AI failed to initialize',
      });
    }
  },

  refreshLlmStatus: async () => {
    const status = await invoke<LocalLlmStatus>('get_local_llm_status');
    set({ realLlmStatus: status });

    const message = status.message.toLowerCase();
    const isTransitioning = !status.available && (
      message.includes('warming') || message.includes('restart')
    );
    const pollGeneration = ++llmStatusPollGeneration;
    if (isTransitioning) {
      void (async () => {
        for (let attempt = 0; attempt < 120; attempt += 1) {
          await new Promise(resolve => window.setTimeout(resolve, 1_000));
          if (pollGeneration !== llmStatusPollGeneration) return;
          try {
            const nextStatus = await invoke<LocalLlmStatus>('get_local_llm_status');
            set({ realLlmStatus: nextStatus });
            const nextMessage = nextStatus.message.toLowerCase();
            if (
              nextStatus.available ||
              (!nextMessage.includes('warming') && !nextMessage.includes('restart'))
            ) {
              return;
            }
          } catch {
            return;
          }
        }
      })();
    }
    return status;
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
        type: 'explanation',
        content: response.text,
        confidence: response.confidence,
        timestamp: Date.now(),
        source: response.source,
      };

      set(state => ({
        suggestions: appendSuggestion(state.suggestions, suggestion),
        isProcessing: false,
      }));
      if (response.source !== 'local_llm') {
        void get().refreshLlmStatus().catch(() => undefined);
      }
    } catch (error) {
      console.error('Failed to get AI suggestions:', error);
      set({ isProcessing: false });
    }
  },

  explainCommand: async (command: string) => {
    try {
      const response = await invoke<AIResponse>('ai_explain_command', { command });
      if (response.source !== 'local_llm') {
        void get().refreshLlmStatus().catch(() => undefined);
      }
      return response;
    } catch (error) {
      console.error('Failed to explain command:', error);
      return { text: 'Unable to explain command', confidence: 0, source: 'unavailable' };
    }
  },

  fixError: async (error: string, command: string, context: string) => {
    try {
      const response = await invoke<AIResponse>('ai_fix_error', {
        errorOutput: error,
        command,
        context,
      });
      if (response.source !== 'local_llm') {
        void get().refreshLlmStatus().catch(() => undefined);
      }
      return response;
    } catch (error) {
      console.error('Failed to fix error:', error);
      return { text: 'Unable to suggest fix', confidence: 0, source: 'unavailable' };
    }
  },

  analyzeOutput: async (output: string, command: string) => {
    try {
      const response = await invoke<AIResponse>('ai_analyze_output', { output, command });
      if (response.source !== 'local_llm') {
        void get().refreshLlmStatus().catch(() => undefined);
      }
      return response;
    } catch (error) {
      console.error('Failed to analyze output:', error);
      return { text: 'Unable to analyze output', confidence: 0, source: 'unavailable' };
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
      return { text: 'Local intelligence is not ready', confidence: 0, source: 'unavailable' };
    }

    set({ isProcessing: true });
    try {
      const response = await invoke<AIResponse>('ai_translate_natural_language', {
        naturalLanguage: text,
        context,
      });

      // Commands are displayed only through create_command_plan so every
      // insertable suggestion carries a backend risk assessment.
      set({ isProcessing: false });
      if (response.source !== 'local_llm') {
        void get().refreshLlmStatus().catch(() => undefined);
      }
      return response;
    } catch (error) {
      console.error('Failed to translate natural language:', error);
      set({ isProcessing: false });
      return { text: 'Unable to translate', confidence: 0, source: 'unavailable' };
    }
  },

  addSuggestion: (suggestion: AISuggestion) => {
    set(state => ({
      suggestions: appendSuggestion(state.suggestions, suggestion)
    }));
  },

  clearSuggestions: () => {
    set({ suggestions: [] });
  },

  updateFeedback: async (suggestionId: string, command: string, feedback: number) => {
    try {
      // Extract the actual command from the suggestion content
      let actualCommand = command;

      // Handle different suggestion formats
      if (command.includes('→')) {
        // Natural language → command format
        actualCommand = command.split('→ ')[1] || command;
      } else if (command.startsWith('💡')) {
        // Remove emoji prefix
        actualCommand = command.replace('💡 Natural Language → Command: ', '');
      }

      // Clean up any remaining formatting
      actualCommand = actualCommand.trim();

      // Feedback adjusts only this running session's local preferences.
      await invoke('update_ai_feedback', { command: actualCommand, feedback });

      // Update the local suggestion state to show feedback visually
      set(state => ({
        suggestions: state.suggestions.map(suggestion =>
          suggestion.id === suggestionId
            ? {
              ...suggestion,
              feedback: feedback > 0.5 ? 'positive' : 'negative' as 'positive' | 'negative'
            }
            : suggestion
        )
      }));
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
}));
