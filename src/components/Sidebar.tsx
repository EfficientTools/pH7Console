import React from 'react';
import { useTerminalStore } from '../store/terminalStore';
import { Terminal, Plus, X, Settings } from 'lucide-react';

export const Sidebar: React.FC = () => {
  const { sessions, activeSession, setActiveSession, createSession } = useTerminalStore();

  const handleCreateSession = async () => {
    await createSession();
  };

  return (
    <div className="w-64 bg-terminal-surface border-r border-terminal-border flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-terminal-border">
        <div className="flex items-center justify-between">
          <h1 className="font-semibold text-terminal-text">Sessions</h1>
          <button
            onClick={handleCreateSession}
            className="p-1 hover:bg-terminal-border rounded transition-colors focus-ring"
            title="New Terminal"
          >
            <Plus className="w-4 h-4 text-terminal-muted" />
          </button>
        </div>
      </div>

      {/* Sessions List */}
      <div className="flex-1 overflow-y-auto">
        {sessions.length === 0 ? (
          <div className="p-4 text-center text-terminal-muted">
            <Terminal className="w-8 h-8 mx-auto mb-2 opacity-50" />
            <p className="text-sm">No terminal sessions</p>
            <button
              onClick={handleCreateSession}
              className="mt-2 text-ai-primary hover:text-ai-primary/80 text-sm"
            >
              Create your first terminal
            </button>
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {sessions.map((session) => (
              <div
                key={session.id}
                onClick={() => setActiveSession(session.id)}
                className={`
                  group flex items-center justify-between p-3 rounded-lg cursor-pointer transition-colors
                  ${session.id === activeSession 
                    ? 'bg-ai-primary text-white' 
                    : 'text-terminal-text hover:bg-terminal-border'
                  }
                `}
              >
                <div className="flex items-center space-x-3 min-w-0">
                  <Terminal className="w-4 h-4 flex-shrink-0" />
                  <div className="min-w-0">
                    <div className="font-medium truncate">{session.title}</div>
                    <div className={`text-xs truncate ${
                      session.id === activeSession ? 'text-white/70' : 'text-terminal-muted'
                    }`}>
                      {session.working_directory}
                    </div>
                  </div>
                </div>
                
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    // Handle close session
                  }}
                  className="opacity-0 group-hover:opacity-100 p-1 hover:bg-black/20 rounded transition-all"
                >
                  <X className="w-3 h-3" />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="p-4 border-t border-terminal-border">
        <button className="w-full flex items-center space-x-2 p-2 text-terminal-muted hover:text-terminal-text hover:bg-terminal-border rounded transition-colors focus-ring">
          <Settings className="w-4 h-4" />
          <span className="text-sm">Settings</span>
        </button>
      </div>
    </div>
  );
};
