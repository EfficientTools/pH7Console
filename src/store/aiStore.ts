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
    try {
      return await invoke<AIResponse>('ai_translate_natural_language', {
        naturalLanguage: text,
        context,
      });
    } catch (error) {
      console.error('Failed to translate natural language:', error);
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
}));
