import { useState, useEffect } from 'react';
import { Terminal } from './components/Terminal';
import { AIPanel } from './components/AIPanel';
import { Sidebar } from './components/Sidebar';
import { useTerminalStore } from './store/terminalStore';
import { useAIStore } from './store/aiStore';
import { PanelLeftOpen, PanelLeftClose, PanelRightOpen, PanelRightClose } from 'lucide-react';

function App() {
  const [isLoading, setIsLoading] = useState(true);
  const [aiModelLoaded, setAiModelLoaded] = useState(false);
  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [aiPanelVisible, setAiPanelVisible] = useState(true);
  const { createSession } = useTerminalStore();
  const { loadModel } = useAIStore();

  useEffect(() => {
    const initializeApp = async () => {
      try {
        // Create initial terminal session
        await createSession('Main Terminal');
        
        // Load AI model
        await loadModel();
        setAiModelLoaded(true);
        
        setIsLoading(false);
        
        // Ensure terminal gets focus after initialization
        setTimeout(() => {
          const terminalInput = document.querySelector('input[type="text"]') as HTMLInputElement;
          if (terminalInput) {
            terminalInput.focus();
          }
        }, 100);
      } catch (error) {
        console.error('Failed to initialize app:', error);
        setIsLoading(false);
      }
    };

    initializeApp();
  }, [createSession, loadModel]);

  // Keyboard shortcuts for toggling panels
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + B to toggle sidebar
      if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
        e.preventDefault();
        setSidebarVisible(prev => {
          const newValue = !prev;
          // Re-focus terminal after animation
          setTimeout(() => {
            const terminalInput = document.querySelector('input[type="text"]') as HTMLInputElement;
            if (terminalInput) {
              terminalInput.focus();
            }
          }, 350); // Wait for transition to complete
          return newValue;
        });
      }
      // Cmd/Ctrl + J to toggle AI panel
      if ((e.metaKey || e.ctrlKey) && e.key === 'j') {
        e.preventDefault();
        setAiPanelVisible(prev => {
          const newValue = !prev;
          // Re-focus terminal after animation
          setTimeout(() => {
            const terminalInput = document.querySelector('input[type="text"]') as HTMLInputElement;
            if (terminalInput) {
              terminalInput.focus();
            }
          }, 350); // Wait for transition to complete
          return newValue;
        });
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);

  if (isLoading) {
    return (
      <div className="h-screen bg-terminal-bg flex items-center justify-center">
        <div className="text-center">
          <div className="ai-badge mb-4">pH7Console</div>
          <div className="text-terminal-muted">Initializing ML-First Terminal...</div>
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
    <div className="h-screen bg-terminal-bg flex">
      {/* Sidebar with transition */}
      <div className={`h-full transition-all duration-300 ease-in-out ${
        sidebarVisible ? 'w-64 opacity-100' : 'w-0 opacity-0 overflow-hidden'
      }`}>
        <Sidebar />
      </div>
      
      {/* Main Content */}
      <div className="flex-1 flex flex-col transition-all duration-300 ease-in-out">
        {/* Header */}
        <header className="bg-terminal-surface border-b border-terminal-border px-6 py-3 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <button
              onClick={() => {
                setSidebarVisible(!sidebarVisible);
                // Re-focus terminal after animation
                setTimeout(() => {
                  const terminalInput = document.querySelector('input[type="text"]') as HTMLInputElement;
                  if (terminalInput) {
                    terminalInput.focus();
                  }
                }, 350);
              }}
              className="p-1 hover:bg-terminal-border rounded transition-colors"
              title={sidebarVisible ? "Hide Sidebar (⌘B)" : "Show Sidebar (⌘B)"}
            >
              {sidebarVisible ? (
                <PanelLeftClose className="w-4 h-4 text-terminal-muted" />
              ) : (
                <PanelLeftOpen className="w-4 h-4 text-terminal-muted" />
              )}
            </button>
            <div className="ai-badge">pH7Console</div>
            <div className="text-terminal-muted text-sm">
              ML-First Terminal {aiModelLoaded && '• AI Ready'}
            </div>
          </div>
          
          <div className="flex items-center space-x-2">
            {aiModelLoaded && (
              <div className="flex items-center space-x-2">
                <div className="w-2 h-2 bg-ai-primary rounded-full animate-pulse-soft"></div>
                <span className="text-xs text-terminal-muted">Local AI Active</span>
              </div>
            )}
            <button
              onClick={() => {
                setAiPanelVisible(!aiPanelVisible);
                // Re-focus terminal after animation
                setTimeout(() => {
                  const terminalInput = document.querySelector('input[type="text"]') as HTMLInputElement;
                  if (terminalInput) {
                    terminalInput.focus();
                  }
                }, 350);
              }}
              className="p-1 hover:bg-terminal-border rounded transition-colors"
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

        {/* Terminal and AI Panel */}
        <div className="flex-1 flex">
          {/* Terminal */}
          <div className="flex-1">
            <Terminal />
          </div>
          
          {/* AI Panel with transition */}
          <div className={`h-full transition-all duration-300 ease-in-out border-l border-terminal-border ${
            aiPanelVisible ? 'w-80 opacity-100' : 'w-0 opacity-0 overflow-hidden border-l-0'
          }`}>
            <AIPanel />
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
