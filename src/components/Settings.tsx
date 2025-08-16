import React, { useState, useRef } from 'react';
import { X, Settings as SettingsIcon, Monitor, Palette, Keyboard, Brain } from 'lucide-react';
import { useClickOutside } from '../hooks/useClickOutside';

interface SettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

export const Settings: React.FC<SettingsProps> = ({ isOpen, onClose }) => {
  const [activeTab, setActiveTab] = useState('appearance');
  const modalRef = useRef<HTMLDivElement>(null);

  // Close modal when clicking outside
  useClickOutside(modalRef, onClose, isOpen);
  const [settings, setSettings] = useState({
    appearance: {
      theme: 'dark',
      fontSize: 14,
      fontFamily: 'SF Mono',
      opacity: 0.95,
    },
    terminal: {
      shell: 'zsh',
      historyLimit: 1000,
      clearOnExit: false,
      saveHistory: true,
    },
    ai: {
      modelSize: 'lightweight',
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
      }
    }
  });

  if (!isOpen) return null;

  const tabs = [
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'terminal', label: 'Terminal', icon: Monitor },
    { id: 'ai', label: 'AI Assistant', icon: Brain },
    { id: 'keyboard', label: 'Keyboard', icon: Keyboard },
  ];

  const updateSetting = (category: string, key: string, value: any) => {
    setSettings(prev => ({
      ...prev,
      [category]: {
        ...prev[category as keyof typeof prev],
        [key]: value,
      }
    }));
  };

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
      <div 
        ref={modalRef}
        className="bg-terminal-surface border border-terminal-border rounded-lg shadow-2xl w-full max-w-4xl h-[80vh] flex"
      >
        {/* Sidebar */}
        <div className="w-64 border-r border-terminal-border">
          <div className="p-4 border-b border-terminal-border">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <SettingsIcon className="w-5 h-5 text-ai-primary" />
                <h2 className="font-semibold text-terminal-text">Settings</h2>
              </div>
              <button
                onClick={onClose}
                className="p-1 hover:bg-terminal-border rounded transition-colors"
              >
                <X className="w-4 h-4 text-terminal-muted" />
              </button>
            </div>
          </div>
          
          <div className="p-2">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`w-full flex items-center space-x-3 p-3 rounded-lg transition-colors text-left ${
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
                    <label className="block text-sm font-medium text-terminal-text mb-2">
                      Theme
                    </label>
                    <select
                      value={settings.appearance.theme}
                      onChange={(e) => updateSetting('appearance', 'theme', e.target.value)}
                      className="w-full p-2 bg-terminal-bg border border-terminal-border rounded focus:ring-2 focus:ring-ai-primary focus:border-transparent"
                    >
                      <option value="dark">Dark</option>
                      <option value="light">Light</option>
                      <option value="auto">Auto (System)</option>
                    </select>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-terminal-text mb-2">
                      Font Size: {settings.appearance.fontSize}px
                    </label>
                    <input
                      type="range"
                      min="10"
                      max="24"
                      value={settings.appearance.fontSize}
                      onChange={(e) => updateSetting('appearance', 'fontSize', parseInt(e.target.value))}
                      className="w-full h-2 bg-terminal-border rounded-lg appearance-none cursor-pointer"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-terminal-text mb-2">
                      Font Family
                    </label>
                    <select
                      value={settings.appearance.fontFamily}
                      onChange={(e) => updateSetting('appearance', 'fontFamily', e.target.value)}
                      className="w-full p-2 bg-terminal-bg border border-terminal-border rounded focus:ring-2 focus:ring-ai-primary focus:border-transparent"
                    >
                      <option value="SF Mono">SF Mono</option>
                      <option value="Monaco">Monaco</option>
                      <option value="Inconsolata">Inconsolata</option>
                      <option value="Roboto Mono">Roboto Mono</option>
                      <option value="Courier New">Courier New</option>
                    </select>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-terminal-text mb-2">
                      Terminal Opacity: {Math.round(settings.appearance.opacity * 100)}%
                    </label>
                    <input
                      type="range"
                      min="0.7"
                      max="1"
                      step="0.05"
                      value={settings.appearance.opacity}
                      onChange={(e) => updateSetting('appearance', 'opacity', parseFloat(e.target.value))}
                      className="w-full h-2 bg-terminal-border rounded-lg appearance-none cursor-pointer"
                    />
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'terminal' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-terminal-text mb-4">Terminal Settings</h3>
                
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-terminal-text mb-2">
                      Default Shell
                    </label>
                    <select
                      value={settings.terminal.shell}
                      onChange={(e) => updateSetting('terminal', 'shell', e.target.value)}
                      className="w-full p-2 bg-terminal-bg border border-terminal-border rounded focus:ring-2 focus:ring-ai-primary focus:border-transparent"
                    >
                      <option value="zsh">Zsh</option>
                      <option value="bash">Bash</option>
                      <option value="fish">Fish</option>
                      <option value="powershell">PowerShell</option>
                    </select>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-terminal-text mb-2">
                      History Limit: {settings.terminal.historyLimit} commands
                    </label>
                    <input
                      type="range"
                      min="100"
                      max="5000"
                      step="100"
                      value={settings.terminal.historyLimit}
                      onChange={(e) => updateSetting('terminal', 'historyLimit', parseInt(e.target.value))}
                      className="w-full h-2 bg-terminal-border rounded-lg appearance-none cursor-pointer"
                    />
                  </div>

                  <div className="space-y-2">
                    <label className="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        checked={settings.terminal.saveHistory}
                        onChange={(e) => updateSetting('terminal', 'saveHistory', e.target.checked)}
                        className="rounded border-terminal-border"
                      />
                      <span className="text-sm text-terminal-text">Save command history between sessions</span>
                    </label>

                    <label className="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        checked={settings.terminal.clearOnExit}
                        onChange={(e) => updateSetting('terminal', 'clearOnExit', e.target.checked)}
                        className="rounded border-terminal-border"
                      />
                      <span className="text-sm text-terminal-text">Clear terminal on exit</span>
                    </label>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'ai' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-terminal-text mb-4">AI Assistant</h3>
                
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-terminal-text mb-2">
                      Model Size
                    </label>
                    <select
                      value={settings.ai.modelSize}
                      onChange={(e) => updateSetting('ai', 'modelSize', e.target.value)}
                      className="w-full p-2 bg-terminal-bg border border-terminal-border rounded focus:ring-2 focus:ring-ai-primary focus:border-transparent"
                    >
                      <option value="lightweight">Lightweight (Fast, Lower Accuracy)</option>
                      <option value="balanced">Balanced (Good Speed & Accuracy)</option>
                      <option value="accurate">Accurate (Slower, Higher Accuracy)</option>
                    </select>
                    <p className="text-xs text-terminal-muted mt-1">
                      Current: Pattern-based AI (Optimal for MacBook Air)
                    </p>
                  </div>

                  <div className="space-y-2">
                    <label className="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        checked={settings.ai.autoSuggestions}
                        onChange={(e) => updateSetting('ai', 'autoSuggestions', e.target.checked)}
                        className="rounded border-terminal-border"
                      />
                      <span className="text-sm text-terminal-text">Auto-generate command suggestions</span>
                    </label>

                    <label className="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        checked={settings.ai.explainOnHover}
                        onChange={(e) => updateSetting('ai', 'explainOnHover', e.target.checked)}
                        className="rounded border-terminal-border"
                      />
                      <span className="text-sm text-terminal-text">Explain commands on hover</span>
                    </label>

                    <label className="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        checked={settings.ai.smartCompletions}
                        onChange={(e) => updateSetting('ai', 'smartCompletions', e.target.checked)}
                        className="rounded border-terminal-border"
                      />
                      <span className="text-sm text-terminal-text">Smart command completions</span>
                    </label>
                  </div>

                  <div className="bg-ai-primary/10 border border-ai-primary/20 rounded-lg p-4">
                    <div className="flex items-center space-x-2 mb-2">
                      <Brain className="w-4 h-4 text-ai-primary" />
                      <span className="text-sm font-medium text-ai-primary">Local AI Benefits</span>
                    </div>
                    <ul className="text-xs text-terminal-text space-y-1">
                      <li>• Privacy: All processing stays on your device</li>
                      <li>• Speed: No network latency for suggestions</li>
                      <li>• Efficiency: Optimized for lightweight devices</li>
                      <li>• Offline: Works without internet connection</li>
                    </ul>
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
                  {Object.entries(settings.keyboard.shortcuts).map(([action, shortcut]) => (
                    <div key={action} className="flex items-center justify-between p-3 bg-terminal-bg rounded-lg border border-terminal-border">
                      <span className="text-sm text-terminal-text capitalize">
                        {action.replace(/([A-Z])/g, ' $1').trim()}
                      </span>
                      <div className="flex items-center space-x-2">
                        <kbd className="px-2 py-1 bg-terminal-border rounded text-xs font-mono">
                          {shortcut}
                        </kbd>
                        <button className="text-xs text-ai-primary hover:text-ai-primary/80">
                          Edit
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
