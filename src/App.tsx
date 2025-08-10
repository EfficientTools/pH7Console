import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Terminal } from './components/Terminal';
import { AIPanel } from './components/AIPanel';
import { Sidebar } from './components/Sidebar';
import { useTerminalStore } from './store/terminalStore';
import { useAIStore } from './store/aiStore';

interface AIResponse {
  text: string;
  confidence: number;
  reasoning?: string;
}

function App() {
  const [isLoading, setIsLoading] = useState(true);
  const [aiModelLoaded, setAiModelLoaded] = useState(false);
  const { activeSession, createSession } = useTerminalStore();
  const { suggestions, loadModel } = useAIStore();

  useEffect(() => {
    const initializeApp = async () => {
      try {
        // Create initial terminal session
        await createSession('Main Terminal');
        
        // Load AI model
        await loadModel();
        setAiModelLoaded(true);
        
        setIsLoading(false);
      } catch (error) {
        console.error('Failed to initialize app:', error);
        setIsLoading(false);
      }
    };

    initializeApp();
  }, [createSession, loadModel]);

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
      {/* Sidebar */}
      <Sidebar />
      
      {/* Main Content */}
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <header className="bg-terminal-surface border-b border-terminal-border px-6 py-3 flex items-center justify-between">
          <div className="flex items-center space-x-3">
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
          </div>
        </header>

        {/* Terminal and AI Panel */}
        <div className="flex-1 flex">
          {/* Terminal */}
          <div className="flex-1">
            <Terminal />
          </div>
          
          {/* AI Panel */}
          <div className="w-80 border-l border-terminal-border">
            <AIPanel />
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
