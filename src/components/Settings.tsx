import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Database, Keyboard, Palette, ShieldCheck, Trash2, X, Settings as SettingsIcon } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useClickOutside } from '../hooks/useClickOutside';
import { useSettingsStore } from '../store/settingsStore';
import { useTerminalStore } from '../store/terminalStore';

interface HistoryPersistenceStatus {
  encryptedPersistence: boolean;
  mode: 'encrypted' | 'memory_only';
  message: string;
}

interface SettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

export const Settings: React.FC<SettingsProps> = ({ isOpen, onClose }) => {
  const [activeTab, setActiveTab] = React.useState('appearance');
  const [historyStatus, setHistoryStatus] = useState<HistoryPersistenceStatus | null>(null);
  const [privacyError, setPrivacyError] = useState<string | null>(null);
  const [isClearingHistory, setIsClearingHistory] = useState(false);
  const modalRef = useRef<HTMLDivElement>(null);

  // Close modal when clicking outside
  useClickOutside(modalRef, onClose, isOpen);

  const { appearance, keyboard, updateAppearance } = useSettingsStore();
  const clearHistory = useTerminalStore(state => state.clearHistory);

  const refreshHistoryStatus = useCallback(async () => {
    try {
      const status = await invoke<HistoryPersistenceStatus>('get_history_persistence_status');
      setHistoryStatus(status);
      setPrivacyError(null);
    } catch {
      setHistoryStatus(null);
      setPrivacyError('History protection status is temporarily unavailable.');
    }
  }, []);

  const handleClearHistory = useCallback(async () => {
    if (!window.confirm('Clear all completed command history from this Mac? This cannot be undone.')) {
      return;
    }

    setIsClearingHistory(true);
    setPrivacyError(null);
    try {
      await clearHistory();
      await refreshHistoryStatus();
    } catch {
      setPrivacyError('Command history could not be cleared. No records were hidden from the app.');
    } finally {
      setIsClearingHistory(false);
    }
  }, [clearHistory, refreshHistoryStatus]);

  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  useEffect(() => {
    if (isOpen && activeTab === 'privacy') {
      void refreshHistoryStatus();
    }
  }, [activeTab, isOpen, refreshHistoryStatus]);

  if (!isOpen) return null;

  const tabs = [
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'keyboard', label: 'Keyboard', icon: Keyboard },
    { id: 'privacy', label: 'Privacy', icon: ShieldCheck },
  ];

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
      <div 
        ref={modalRef}
        className="bg-terminal-surface border border-terminal-border rounded-xl shadow-2xl w-full max-w-4xl h-[80vh] max-h-[760px] flex overflow-hidden"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        {/* Sidebar */}
        <div className="w-64 border-r border-terminal-border">
          <div className="p-4 border-b border-terminal-border">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <SettingsIcon className="w-5 h-5 text-ai-primary" />
                <h2 id="settings-title" className="font-semibold text-terminal-text">Settings</h2>
              </div>
              <button
                type="button"
                onClick={onClose}
                aria-label="Close settings"
                className="p-1 hover:bg-terminal-border rounded transition-colors"
              >
                <X className="w-4 h-4 text-terminal-muted" />
              </button>
            </div>
          </div>
          
          <div className="p-2" role="tablist" aria-label="Settings sections">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              return (
                <button
                  type="button"
                  role="tab"
                  aria-selected={activeTab === tab.id}
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`w-full flex items-center space-x-3 p-3 rounded-lg transition-colors text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-ai-primary ${
                    activeTab === tab.id
                      ? 'bg-ai-primary text-white'
                      : 'text-terminal-text hover:bg-terminal-border'
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  <span className="text-sm">{tab.label}</span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 p-6 overflow-y-auto">
          {activeTab === 'appearance' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-terminal-text mb-4">Appearance</h3>
                
                <div className="space-y-4">
                  <div>
                    <label htmlFor="terminal-font-size" className="block text-sm font-medium text-terminal-text mb-2">
                      Font Size: {appearance.fontSize}px
                    </label>
                    <input
                      id="terminal-font-size"
                      type="range"
                      min="10"
                      max="24"
                      value={appearance.fontSize}
                      onChange={(e) => updateAppearance('fontSize', parseInt(e.target.value, 10))}
                      className="w-full h-2 bg-terminal-border rounded-lg appearance-none cursor-pointer"
                      aria-valuetext={`${appearance.fontSize} pixels`}
                    />
                  </div>

                  <div>
                    <label htmlFor="terminal-font-family" className="block text-sm font-medium text-terminal-text mb-2">
                      Font Family
                    </label>
                    <select
                      id="terminal-font-family"
                      value={appearance.fontFamily}
                      onChange={(e) => updateAppearance('fontFamily', e.target.value)}
                      className="w-full p-2 bg-terminal-bg border border-terminal-border rounded focus:ring-2 focus:ring-ai-primary focus:border-transparent font-mono"
                    >
                      <option value="ui-monospace">System Default (SF Mono / Cascadia Code)</option>
                      <option value="SF Mono">SF Mono</option>
                      <option value="Monaco">Monaco</option>
                      <option value="Menlo">Menlo</option>
                      <option value="Inconsolata">Inconsolata</option>
                      <option value="JetBrains Mono">JetBrains Mono</option>
                      <option value="Fira Code">Fira Code</option>
                      <option value="Roboto Mono">Roboto Mono</option>
                      <option value="Courier New">Courier New</option>
                    </select>
                    <p className="text-xs text-terminal-muted mt-1">
                      Font must be installed on your system to take effect.
                    </p>
                  </div>

                </div>
              </div>
            </div>
          )}

          {activeTab === 'keyboard' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-terminal-text mb-4">Keyboard Shortcuts</h3>
                
                <div className="space-y-3">
                  {Object.entries(keyboard.shortcuts).map(([action, shortcut]) => (
                    <div key={action} className="flex items-center justify-between p-3 bg-terminal-bg rounded-lg border border-terminal-border">
                      <span className="text-sm text-terminal-text capitalize">
                        {action === 'toggleAI'
                          ? 'Toggle AI'
                          : action.replace(/([A-Z])/g, ' $1').trim()}
                      </span>
                      <kbd className="px-2 py-1 bg-terminal-border rounded text-xs font-mono">
                        {shortcut}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {activeTab === 'privacy' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-terminal-text">Privacy</h3>
                <p className="mt-1 text-sm text-terminal-muted">
                  pH7Console has no account, telemetry, advertising, or remote AI service. Terminal
                  context and local-model prompts stay on this Mac.
                </p>
              </div>

              <section className="rounded-lg border border-terminal-border bg-terminal-bg p-4" aria-labelledby="history-protection-title">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex min-w-0 items-start gap-3">
                    <Database className="mt-0.5 h-5 w-5 flex-none text-ai-primary" />
                    <div>
                      <h4 id="history-protection-title" className="font-medium text-terminal-text">
                        Command history
                      </h4>
                      <p className="mt-1 text-sm text-terminal-muted">
                        Command text and execution metadata are redacted, bounded, and never include
                        command output. Persistent history is encrypted with SQLCipher using a random
                        key protected by the macOS Keychain.
                      </p>
                    </div>
                  </div>
                  {historyStatus && (
                    <span
                      className={`flex-none rounded-full border px-2 py-1 text-xs ${
                        historyStatus.encryptedPersistence
                          ? 'border-green-400/30 bg-green-400/10 text-green-300'
                          : 'border-amber-400/30 bg-amber-400/10 text-amber-300'
                      }`}
                    >
                      {historyStatus.message}
                    </span>
                  )}
                </div>

                <div className="mt-4 flex items-center justify-between gap-4 border-t border-terminal-border pt-4">
                  <p className="text-xs text-terminal-muted">
                    Clear removes completed command records from both searchable and in-memory history.
                  </p>
                  <button
                    type="button"
                    onClick={() => void handleClearHistory()}
                    disabled={isClearingHistory}
                    className="inline-flex flex-none items-center gap-2 rounded border border-red-400/30 px-3 py-2 text-sm text-red-300 transition-colors hover:bg-red-400/10 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    <Trash2 className="h-4 w-4" />
                    {isClearingHistory ? 'Clearing…' : 'Clear history'}
                  </button>
                </div>
              </section>

              <section className="rounded-lg border border-terminal-border bg-terminal-bg p-4" aria-labelledby="local-intelligence-title">
                <div className="flex items-start gap-3">
                  <ShieldCheck className="mt-0.5 h-5 w-5 flex-none text-ai-primary" />
                  <div>
                    <h4 id="local-intelligence-title" className="font-medium text-terminal-text">
                      Local intelligence
                    </h4>
                    <p className="mt-1 text-sm text-terminal-muted">
                      The bundled coder model runs through an authenticated loopback-only helper.
                      Suggestion adaptation is held in memory for the current app session and is not
                      written to disk. Generated commands are always shown for review before they can run.
                    </p>
                  </div>
                </div>
              </section>

              <div className="rounded-lg border border-terminal-border px-4 py-3 text-sm text-terminal-muted">
                <p>
                  Commands you choose to run can still read files, change data, or contact network
                  services according to macOS permissions. The Mac App Store edition limits workspace
                  access to folders you explicitly select.
                </p>
                <p className="mt-2">
                  Privacy questions: <a className="text-ai-primary hover:underline" href="mailto:pierre@ph7.me">pierre@ph7.me</a>
                </p>
              </div>

              {privacyError && (
                <p role="alert" className="text-sm text-red-300">{privacyError}</p>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
