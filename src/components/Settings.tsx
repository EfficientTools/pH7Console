import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Database, Keyboard, Palette, ShieldCheck, Trash2, X, Settings as SettingsIcon } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useClickOutside } from '../hooks/useClickOutside';
import { useAIStore } from '../store/aiStore';
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
  const { isModelLoaded, isProcessing, realLlmStatus } = useAIStore();
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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-3 backdrop-blur-sm sm:p-6">
      <div 
        ref={modalRef}
        className="flex max-h-[calc(100vh-1.5rem)] min-h-0 w-full max-w-4xl flex-col overflow-hidden rounded-xl border border-terminal-border bg-terminal-surface shadow-2xl sm:h-[80vh] sm:max-h-[760px] sm:flex-row"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        {/* Sidebar */}
        <div className="w-full shrink-0 border-b border-terminal-border sm:w-56 sm:border-b-0 sm:border-r md:w-64">
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
          
          <div className="flex gap-1 overflow-x-auto p-2 sm:block" role="tablist" aria-label="Settings sections">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              return (
                <button
                  type="button"
                  role="tab"
                  aria-selected={activeTab === tab.id}
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`flex min-w-fit flex-1 items-center justify-center space-x-2 rounded-lg p-2.5 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ai-primary sm:mb-1 sm:w-full sm:justify-start sm:space-x-3 sm:p-3 ${
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
        <div className="min-h-0 min-w-0 flex-1 overflow-y-auto p-4 sm:p-6">
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
                <div className="flex flex-col items-start gap-4 sm:flex-row sm:justify-between">
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

                <div className="mt-4 flex flex-col items-start gap-4 border-t border-terminal-border pt-4 sm:flex-row sm:items-center sm:justify-between">
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
                  <div className="min-w-0">
                    <h4 id="local-intelligence-title" className="font-medium text-terminal-text">
                      Authenticated loopback inference
                    </h4>
                    <p className="mt-1 text-sm text-terminal-muted">
                      The bundled coder model runs in a signed helper that listens only on
                      <code className="mx-1 rounded bg-terminal-border/60 px-1 py-0.5 font-mono text-xs text-terminal-text">127.0.0.1</code>
                      using an operating-system-selected ephemeral port. Every launch uses a new random
                      API key, and pH7Console is the only client. The helper never binds to a LAN or
                      public interface.
                    </p>
                    <p className="mt-2 text-sm text-terminal-muted">
                      The helper inherits the parent App Sandbox, so the parent requires Incoming
                      Connections (<code className="break-all font-mono text-xs text-terminal-text">com.apple.security.network.server</code>)
                      for this on-device inference socket. Removing it prevents the local model from
                      accepting pH7Console’s loopback requests.
                    </p>
                    <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
                      <span
                        className={`rounded-full border px-2 py-1 ${
                          realLlmStatus.available
                            ? 'border-green-400/30 bg-green-400/10 text-green-300'
                            : isProcessing
                              ? 'border-amber-400/30 bg-amber-400/10 text-amber-300'
                              : 'border-sky-400/30 bg-sky-400/10 text-sky-300'
                        }`}
                        role="status"
                      >
                        {realLlmStatus.available
                          ? 'On-device model ready'
                          : isProcessing
                            ? 'On-device model warming'
                            : isModelLoaded
                              ? 'Deterministic fallback ready'
                              : 'Command planning available'}
                      </span>
                      <span className="break-words text-terminal-muted">{realLlmStatus.message}</span>
                    </div>
                    <p className="mt-3 text-xs leading-5 text-terminal-muted">
                      To verify: return to the main window, enter “show the current git branch,” then
                      choose Create Safe Command Plan. Once warm-up completes, the result is labeled
                      “On-device model.”
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
