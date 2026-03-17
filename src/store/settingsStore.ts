import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface AppearanceSettings {
  theme: 'dark' | 'light' | 'auto';
  fontSize: number;
  fontFamily: string;
  opacity: number;
}

export interface TerminalSettings {
  shell: string;
  historyLimit: number;
  clearOnExit: boolean;
  saveHistory: boolean;
}

export interface AISettings {
  modelSize: 'lightweight' | 'balanced' | 'capable';
  autoSuggestions: boolean;
  explainOnHover: boolean;
  smartCompletions: boolean;
}

export interface KeyboardSettings {
  shortcuts: {
    newTerminal: string;
    closeTerminal: string;
    toggleAI: string;
    clearTerminal: string;
  };
}

interface SettingsState {
  appearance: AppearanceSettings;
  terminal: TerminalSettings;
  ai: AISettings;
  keyboard: KeyboardSettings;

  updateAppearance: (key: keyof AppearanceSettings, value: AppearanceSettings[keyof AppearanceSettings]) => void;
  updateTerminal: (key: keyof TerminalSettings, value: TerminalSettings[keyof TerminalSettings]) => void;
  updateAI: (key: keyof AISettings, value: AISettings[keyof AISettings]) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set: (fn: (state: SettingsState) => Partial<SettingsState>) => void) => ({
      appearance: {
        theme: 'dark' as const,
        fontSize: 14,
        // ui-monospace → SF Mono on macOS, Cascadia Code on Windows, system monospace elsewhere
        fontFamily: 'ui-monospace',
        opacity: 0.95,
      },
      terminal: {
        shell: 'zsh',
        historyLimit: 1000,
        clearOnExit: false,
        saveHistory: true,
      },
      ai: {
        modelSize: 'lightweight' as const,
        autoSuggestions: true,
        explainOnHover: true,
        smartCompletions: true,
      },
      keyboard: {
        shortcuts: {
          newTerminal: 'Cmd+T',
          closeTerminal: 'Cmd+W',
          toggleAI: 'Cmd+K',
          clearTerminal: 'Cmd+L',
        },
      },

      updateAppearance: (key: keyof AppearanceSettings, value: AppearanceSettings[keyof AppearanceSettings]) =>
        set((state: SettingsState) => ({
          appearance: { ...state.appearance, [key]: value },
        })),

      updateTerminal: (key: keyof TerminalSettings, value: TerminalSettings[keyof TerminalSettings]) =>
        set((state: SettingsState) => ({
          terminal: { ...state.terminal, [key]: value },
        })),

      updateAI: (key: keyof AISettings, value: AISettings[keyof AISettings]) =>
        set((state: SettingsState) => ({
          ai: { ...state.ai, [key]: value },
        })),
    }),
    {
      name: 'ph7console-settings',
    }
  )
);
