import { lazy, Suspense, useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
import { Terminal } from './components/Terminal';
import { Sidebar } from './components/Sidebar';
import { useTerminalStore } from './store/terminalStore';
import { useAIStore } from './store/aiStore';
import { useSettingsStore } from './store/settingsStore';
import { parseExternalTerminalUrl } from './utils/deepLink';
import {
  PanelLeftOpen,
  PanelLeftClose,
  PanelRightOpen,
  PanelRightClose,
  TerminalSquare,
} from 'lucide-react';

const AIPanel = lazy(() =>
  import('./components/AIPanel').then(module => ({ default: module.AIPanel })),
);

const focusTerminal = () => {
  window.dispatchEvent(new CustomEvent('ph7-focus-terminal'));
};

function App() {
  const [isLoading, setIsLoading] = useState(true);
  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [aiPanelVisible, setAiPanelVisible] = useState(true);
  const {
    activeSession,
    closeSession,
    createSession,
    initializeDefaultSessions,
    sessions,
    setActiveSession,
    updateSessionWorkingDirectory,
  } = useTerminalStore();
  const { loadModel, isModelLoaded, realLlmStatus } = useAIStore();
  const { appearance } = useSettingsStore();

  // Terminal typography is independent from the native-looking Mac interface.
  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty('--terminal-font', `${appearance.fontFamily}, ui-monospace, monospace`);
    root.style.setProperty('--terminal-font-size', `${appearance.fontSize}px`);
  }, [appearance.fontFamily, appearance.fontSize]);

  useEffect(() => {
    const initializeApp = async () => {
      try {
        // Initialize or restore terminal sessions.
        await initializeDefaultSessions();
        
        setIsLoading(false);

        // Terminal readiness is never gated on model initialization. A real
        // local model can be large, so assistance warms in the background.
        void loadModel().catch(error => {
          console.error('Local command intelligence failed to initialize:', error);
        });
        setTimeout(focusTerminal, 100);
      } catch (error) {
        console.error('Failed to initialize app:', error);
        setIsLoading(false);
      }
    };

    initializeApp();
  }, [initializeDefaultSessions, loadModel]);

  useEffect(() => {
    if (isLoading) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
    const recentlyHandledUrls = new Map<string, number>();
    let recentLaunches: number[] = [];

    const handleUrls = async (urls: string[]) => {
      for (const value of urls) {
        if (disposed) return;
        const now = Date.now();
        recentLaunches = recentLaunches.filter(timestamp => now - timestamp < 5_000);
        const previous = recentlyHandledUrls.get(value);
        if (previous !== undefined && now - previous < 3_000) continue;
        if (recentLaunches.length >= 4) {
          console.error('Ignored excessive pH7Console launch requests');
          continue;
        }
        for (const [url, timestamp] of recentlyHandledUrls) {
          if (now - timestamp >= 10_000) recentlyHandledUrls.delete(url);
        }
        recentlyHandledUrls.set(value, now);
        recentLaunches.push(now);
        try {
          const request = parseExternalTerminalUrl(value);
          if (!request) continue;
          const sessionId = await createSession('External Terminal');
          if (!sessionId) continue;

          if (request.workingDirectory) {
            try {
              const resolvedPath = await invoke<string>('change_directory', {
                sessionId,
                newPath: request.workingDirectory,
              });
              updateSessionWorkingDirectory(sessionId, resolvedPath);
            } catch (error) {
              console.error('The requested terminal workspace is unavailable:', error);
            }
          }

          // External launchers may prefill a command, but never append Enter.
          // The user must review and explicitly execute it in the terminal.
          if (request.command) {
            await invoke('write_to_terminal', { sessionId, data: request.command });
          }
          setTimeout(focusTerminal, 0);
        } catch (error) {
          console.error('Ignored invalid pH7Console launch URL:', error);
        }
      }
    };

    void getCurrent().then(urls => {
      if (urls) void handleUrls(urls);
    });
    void onOpenUrl(urls => {
      void handleUrls(urls);
    }).then(stop => {
      if (disposed) stop();
      else stopListening = stop;
    });

    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [createSession, isLoading, updateSessionWorkingDirectory]);

  // Keyboard shortcuts for toggling panels
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + B to toggle sidebar
      if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
        e.preventDefault();
        setSidebarVisible(prev => {
          const newValue = !prev;
          setTimeout(focusTerminal, 350);
          return newValue;
        });
      }
      // Cmd/Ctrl + J to toggle AI panel
      if ((e.metaKey || e.ctrlKey) && e.key === 'j') {
        e.preventDefault();
        setAiPanelVisible(prev => {
          const newValue = !prev;
          setTimeout(focusTerminal, 350);
          return newValue;
        });
      }
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === 't') {
        e.preventDefault();
        void createSession(`Terminal ${sessions.length + 1}`);
      }
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === 'w' && activeSession) {
        e.preventDefault();
        void closeSession(activeSession);
      }
      if ((e.metaKey || e.ctrlKey) && /^[1-9]$/.test(e.key)) {
        const session = sessions[Number(e.key) - 1];
        if (session) {
          e.preventDefault();
          setActiveSession(session.id);
          setTimeout(focusTerminal, 0);
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [activeSession, closeSession, createSession, sessions, setActiveSession]);

  if (isLoading) {
    return (
      <div className="h-screen bg-terminal-bg flex items-center justify-center">
        <div className="text-center" role="status" aria-live="polite">
          <div className="ai-badge mb-4">pH7Console</div>
          <div className="text-terminal-muted">Preparing your private command console...</div>
          <div className="mt-4">
            <div className="w-32 h-2 bg-terminal-border rounded-full mx-auto overflow-hidden">
              <div className="h-full bg-gradient-to-r from-ai-primary to-ai-secondary rounded-full animate-pulse"></div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen bg-terminal-bg flex flex-col">
      {/* Header */}
      <header className="h-11 shrink-0 bg-terminal-surface border-b border-terminal-border px-3 flex items-center justify-between gap-3">
        <div className="min-w-0 flex items-center gap-2.5">
          <button
            type="button"
            onClick={() => {
              setSidebarVisible(!sidebarVisible);
              setTimeout(focusTerminal, 350);
            }}
            className="terminal-toolbar-button"
            aria-controls="ph7-sidebar"
            aria-expanded={sidebarVisible}
            aria-label={sidebarVisible ? 'Hide Sidebar (⌘B)' : 'Show Sidebar (⌘B)'}
            title={sidebarVisible ? "Hide Sidebar (⌘B)" : "Show Sidebar (⌘B)"}
          >
            {sidebarVisible ? (
              <PanelLeftClose className="w-4 h-4 text-terminal-muted" />
            ) : (
              <PanelLeftOpen className="w-4 h-4 text-terminal-muted" />
            )}
          </button>
          <div className="flex min-w-0 items-center gap-2" aria-label="pH7Console">
            <span className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md border border-emerald-400/30 bg-emerald-400/10">
              <TerminalSquare className="h-4 w-4 text-emerald-400" aria-hidden="true" />
            </span>
            <span className="truncate text-sm font-semibold tracking-tight text-terminal-text">pH7Console</span>
            <span className="hidden lg:inline text-xs text-terminal-muted">Private command console</span>
          </div>
        </div>
        
        <div className="flex shrink-0 items-center gap-2">
          {isModelLoaded && (
            <div
              className="flex items-center gap-2 rounded-full border border-terminal-border bg-terminal-bg/60 px-2.5 py-1"
              role="status"
              aria-live="polite"
              title={realLlmStatus.available ? 'Private local language model ready' : 'Private local command intelligence ready'}
            >
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.55)]" aria-hidden="true" />
              <span className="hidden sm:inline text-[11px] font-medium text-terminal-muted">
                {realLlmStatus.available ? 'Local LLM' : 'Local intelligence'}
              </span>
            </div>
          )}
          <button
            type="button"
            onClick={() => {
              setAiPanelVisible(!aiPanelVisible);
              setTimeout(focusTerminal, 350);
            }}
            className="terminal-toolbar-button"
            aria-controls="ph7-ai-panel"
            aria-expanded={aiPanelVisible}
            aria-label={aiPanelVisible ? 'Hide AI Panel (⌘J)' : 'Show AI Panel (⌘J)'}
            title={aiPanelVisible ? "Hide AI Panel (⌘J)" : "Show AI Panel (⌘J)"}
          >
            {aiPanelVisible ? (
              <PanelRightClose className="w-4 h-4 text-terminal-muted" />
            ) : (
              <PanelRightOpen className="w-4 h-4 text-terminal-muted" />
            )}
          </button>
        </div>
      </header>

      {/* Main Content Area - Now flex-1 to take remaining height after header */}
      <div className="flex-1 flex min-h-0">
        {/* Sidebar with transition */}
        <div id="ph7-sidebar" className={`h-full transition-all duration-300 ease-in-out ${
          sidebarVisible ? 'w-64 opacity-100' : 'w-0 opacity-0 overflow-hidden'
        }`} aria-hidden={!sidebarVisible}>
          {sidebarVisible && <Sidebar />}
        </div>
        
        {/* Terminal and AI Panel */}
        <div className="flex-1 flex min-h-0">
          {/* Terminal */}
          <div className="flex-1 min-h-0">
            <Terminal />
          </div>
          
          {/* AI Panel with transition */}
          <div id="ph7-ai-panel" className={`h-full transition-all duration-300 ease-in-out border-l border-terminal-border ${
            aiPanelVisible ? 'w-80 opacity-100' : 'w-0 opacity-0 overflow-hidden border-l-0'
          }`} aria-hidden={!aiPanelVisible}>
            {aiPanelVisible && (
              <Suspense
                fallback={(
                  <div className="flex h-full items-center justify-center bg-terminal-surface text-xs text-terminal-muted" role="status">
                    Loading local intelligence…
                  </div>
                )}
              >
                <AIPanel />
              </Suspense>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
