import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface AppearanceSettings {
  fontSize: number;
  fontFamily: string;
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
  keyboard: KeyboardSettings;

  updateAppearance: (key: keyof AppearanceSettings, value: AppearanceSettings[keyof AppearanceSettings]) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set: (fn: (state: SettingsState) => Partial<SettingsState>) => void) => ({
      appearance: {
        fontSize: 14,
        // ui-monospace → SF Mono on macOS, Cascadia Code on Windows, system monospace elsewhere
        fontFamily: 'ui-monospace',
      },
      keyboard: {
        shortcuts: {
          newTerminal: 'Cmd+T',
          closeTerminal: 'Cmd+W',
          toggleAI: 'Cmd+J',
          clearTerminal: 'Cmd+K',
        },
      },

      updateAppearance: (key: keyof AppearanceSettings, value: AppearanceSettings[keyof AppearanceSettings]) =>
        set((state: SettingsState) => ({
          appearance: { ...state.appearance, [key]: value },
        })),
    }),
    {
      name: 'ph7console-settings',
    }
  )
);
