import React, { useState, useEffect } from 'react';
import { useTerminalStore } from '../store/terminalStore';
import { Terminal, Plus, X, Settings as SettingsIcon } from 'lucide-react';
import { Settings } from './Settings';

export const Sidebar: React.FC = () => {
  const { sessions, activeSession, setActiveSession, createSession, closeSession, updateSessionTitle } = useTerminalStore();
  const [showSettings, setShowSettings] = useState(false);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState('');

  const handleCreateSession = async () => {
    // Generate a unique name based on session count
    const sessionCount = sessions.length + 1;
    await createSession(`Terminal ${sessionCount}`);
  };

  const handleSettingsClick = () => {
    setShowSettings(true);
  };

  const handleCloseSession = async (sessionId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    await closeSession(sessionId);
  };

  const handleDoubleClick = (session: { id: string; title: string }) => {
    setEditingSessionId(session.id);
    setEditingTitle(session.title);
  };

  const handleTitleSubmit = async (sessionId: string) => {
    if (editingTitle.trim() && editingTitle !== sessions.find(s => s.id === sessionId)?.title) {
      await updateSessionTitle(sessionId, editingTitle.trim());
    }
    setEditingSessionId(null);
    setEditingTitle('');
  };

  const handleTitleKeyDown = (e: React.KeyboardEvent, sessionId: string) => {
    // Allow standard OS shortcuts to work (CMD/CTRL + A, C, X, V, Z)
    if ((e.metaKey || e.ctrlKey) && ['a', 'c', 'x', 'v', 'z'].includes(e.key.toLowerCase())) {
      // Let the default behavior happen for OS shortcuts
      return;
    }
    
    // Allow arrow keys, delete, backspace, etc.
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Delete', 'Backspace', 'Home', 'End'].includes(e.key)) {
      // Let the default behavior happen for navigation keys
      return;
    }
    
    if (e.key === 'Enter') {
      e.preventDefault();
      handleTitleSubmit(sessionId);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setEditingSessionId(null);
      setEditingTitle('');
    }
  };

  const handleInputFocus = (e: React.FocusEvent<HTMLInputElement>) => {
    // Select all text when input gets focus for easier editing
    e.target.select();
  };

  const handleInputClick = (e: React.MouseEvent) => {
    // Stop propagation to prevent session selection when clicking on input
    e.stopPropagation();
  };

  // Handle global keyboard events for editing active session
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      // Handle Enter key for editing active session
      if (e.key === 'Enter' && !editingSessionId && activeSession) {
        // Check if the focus is not on an input, button, or other interactive element
        const activeElement = document.activeElement;
        if (!activeElement || 
            (activeElement.tagName !== 'INPUT' && 
             activeElement.tagName !== 'BUTTON' && 
             activeElement.tagName !== 'TEXTAREA' &&
             !activeElement.hasAttribute('contenteditable'))) {
          e.preventDefault();
          const session = sessions.find(s => s.id === activeSession);
          if (session) {
            handleDoubleClick(session);
          }
        }
      }
      
      // Handle arrow keys for session navigation
      if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && !editingSessionId && sessions.length > 1) {
        const activeElement = document.activeElement;
        if (!activeElement || 
            (activeElement.tagName !== 'INPUT' && 
             activeElement.tagName !== 'BUTTON' && 
             activeElement.tagName !== 'TEXTAREA' &&
             !activeElement.hasAttribute('contenteditable'))) {
          e.preventDefault();
          
          const currentIndex = sessions.findIndex(s => s.id === activeSession);
          if (currentIndex !== -1) {
            let newIndex;
            if (e.key === 'ArrowUp') {
              newIndex = currentIndex > 0 ? currentIndex - 1 : sessions.length - 1;
            } else {
              newIndex = currentIndex < sessions.length - 1 ? currentIndex + 1 : 0;
            }
            setActiveSession(sessions[newIndex].id);
          }
        }
      }
    };

    document.addEventListener('keydown', handleGlobalKeyDown);
    return () => {
      document.removeEventListener('keydown', handleGlobalKeyDown);
    };
  }, [editingSessionId, activeSession, sessions, handleDoubleClick, setActiveSession]);

  return (
    <>
      <div className="w-full h-full bg-terminal-surface border-r border-terminal-border flex flex-col">
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
              <p className="text-sm mb-3">No terminal sessions</p>
              <button
                onClick={handleCreateSession}
                className="px-4 py-2 bg-ai-primary text-white rounded-lg hover:bg-ai-primary/90 transition-colors text-sm font-medium"
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
                  onDoubleClick={() => handleDoubleClick(session)}
                  className={`
                    group flex items-center justify-between p-3 rounded-lg cursor-pointer transition-colors
                    ${session.id === activeSession 
                      ? 'bg-ai-primary text-white' 
                      : 'text-terminal-text hover:bg-terminal-border'
                    }
                  `}
                >
                  <div className="flex items-center space-x-3 min-w-0 flex-1">
                    <Terminal className="w-4 h-4 flex-shrink-0" />
                    <div className="min-w-0 flex-1">
                      {editingSessionId === session.id ? (
                        <input
                          type="text"
                          value={editingTitle}
                          onChange={(e) => setEditingTitle(e.target.value)}
                          onBlur={() => handleTitleSubmit(session.id)}
                          onKeyDown={(e) => handleTitleKeyDown(e, session.id)}
                          onFocus={handleInputFocus}
                          onClick={handleInputClick}
                          className="w-full bg-white/10 border border-white/20 rounded px-2 py-1 outline-none font-medium text-current focus:bg-white/20 focus:border-white/40"
                          autoFocus
                        />
                      ) : (
                        <div 
                          className="font-medium truncate" 
                          title={`${session.title} - Press Enter to rename, ↑/↓ arrows to navigate`}
                        >
                          {session.title}
                        </div>
                      )}
                      <div className={`text-xs truncate ${
                        session.id === activeSession ? 'text-white/70' : 'text-terminal-muted'
                      }`}>
                        {session.working_directory}
                      </div>
                    </div>
                  </div>
                  
                  {sessions.length > 1 && (
                    <button
                      onClick={(e) => handleCloseSession(session.id, e)}
                      className="opacity-0 group-hover:opacity-100 p-1 hover:bg-black/20 rounded transition-all"
                      title="Close session"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-terminal-border">
          <button 
            onClick={handleSettingsClick}
            className="w-full flex items-center space-x-2 p-2 text-terminal-muted hover:text-terminal-text hover:bg-terminal-border rounded transition-colors focus-ring"
          >
            <SettingsIcon className="w-4 h-4" />
            <span className="text-sm">Settings</span>
          </button>
        </div>
      </div>

      {/* Settings Modal */}
      <Settings 
        isOpen={showSettings} 
        onClose={() => setShowSettings(false)} 
      />
    </>
  );
};
