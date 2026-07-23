import React, { useState, useEffect, useCallback } from 'react';
import { useTerminalStore } from '../store/terminalStore';
import { message } from '@tauri-apps/plugin-dialog';
import { Terminal, Plus, X, Settings as SettingsIcon, Folder, FolderOpen } from 'lucide-react';
import { Settings } from './Settings';
import { FileExplorer } from './FileExplorer';

export const Sidebar: React.FC = () => {
  const { sessions, activeSession, setActiveSession, createSession, closeSession, updateSessionTitle, selectWorkspace } = useTerminalStore();
  const [showSettings, setShowSettings] = useState(false);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState('');
  const [activeTab, setActiveTab] = useState<'terminals' | 'explorer'>('terminals');
  const [currentExplorerPath, setCurrentExplorerPath] = useState('~');

  useEffect(() => {
    const activeWorkingDirectory = sessions.find(session => session.id === activeSession)?.working_directory;
    setCurrentExplorerPath(activeWorkingDirectory ?? '~');
  }, [activeSession, sessions]);

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

  const handleDoubleClick = useCallback((session: { id: string; title: string }) => {
    setEditingSessionId(session.id);
    setEditingTitle(session.title);
  }, []);

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
      if (showSettings) return;

      // FIRST: Check if ANY input field has focus and skip ALL arrow key handling
      if ((e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
        const activeElement = document.activeElement;
        
        // If ANY input or textarea is focused, completely ignore arrow keys
        if (activeElement && (activeElement.tagName === 'INPUT' || activeElement.tagName === 'TEXTAREA')) {
          return; // Exit immediately, don't process any arrow key logic
        }
      }

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
        
        // VERY specific check for terminal input - be extremely conservative
        const isTerminalInput = activeElement && (
          activeElement.classList.contains('terminal-input') ||
          activeElement.tagName === 'INPUT' && activeElement.closest('.terminal')
        );
        
        // If terminal input is focused, NEVER interfere with arrow keys
        if (isTerminalInput) {
          return; // Don't prevent default, don't capture - let Terminal handle it
        }
        
        // Only capture if we're sure it's NOT terminal input
        e.preventDefault();
        e.stopPropagation(); // Stop event from continuing

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
    };

    document.addEventListener('keydown', handleGlobalKeyDown, false); // Use bubble phase, not capture
    return () => {
      document.removeEventListener('keydown', handleGlobalKeyDown, false);
    };
  }, [editingSessionId, activeSession, sessions, handleDoubleClick, setActiveSession, showSettings]);

  return (
    <>
      <div className="w-full h-full bg-terminal-surface border-r border-terminal-border flex flex-col">
        {/* Header with Tabs */}
        <div className="p-4 border-b border-terminal-border">
          <div className="flex items-center justify-between mb-3">
            <h1 className="font-semibold text-terminal-text">pH7Console</h1>
            {activeTab === 'terminals' && (
              <button
                type="button"
                onClick={handleCreateSession}
                className="p-1 hover:bg-terminal-border rounded transition-colors focus-ring"
                title="New terminal (⌘T)"
                aria-label="New terminal"
              >
                <Plus className="w-4 h-4 text-terminal-muted" />
              </button>
            )}
          </div>
          
          {/* Tab Navigation */}
          <div className="flex bg-terminal-bg rounded-lg p-1" role="tablist" aria-label="Sidebar view">
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'terminals'}
              onClick={() => setActiveTab('terminals')}
              className={`flex-1 flex items-center justify-center gap-2 px-3 py-1.5 rounded text-sm font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ai-primary ${
                activeTab === 'terminals'
                  ? 'bg-ai-primary text-white'
                  : 'text-terminal-muted hover:text-terminal-text hover:bg-terminal-border'
              }`}
            >
              <Terminal className="w-3 h-3" />
              Terminals
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'explorer'}
              onClick={() => setActiveTab('explorer')}
              className={`flex-1 flex items-center justify-center gap-2 px-3 py-1.5 rounded text-sm font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ai-primary ${
                activeTab === 'explorer'
                  ? 'bg-ai-primary text-white'
                  : 'text-terminal-muted hover:text-terminal-text hover:bg-terminal-border'
              }`}
            >
              <Folder className="w-3 h-3" />
              Explorer
            </button>
          </div>
        </div>

        {/* Tab Content */}
        <div className="flex-1 overflow-hidden">
          {activeTab === 'terminals' ? (
            /* Sessions List */
            <div className="h-full overflow-y-auto">
              {sessions.length === 0 ? (
                <div className="p-4 text-center text-terminal-muted">
                  <Terminal className="w-8 h-8 mx-auto mb-2 opacity-50" />
                  <p className="text-sm mb-3">No terminal sessions</p>
                  <button
                    type="button"
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
                      onKeyDown={(event) => {
                        if (event.target !== event.currentTarget) return;
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          event.stopPropagation();
                          setActiveSession(session.id);
                        } else if (event.key === 'F2') {
                          event.preventDefault();
                          event.stopPropagation();
                          handleDoubleClick(session);
                        }
                      }}
                      role="button"
                      tabIndex={0}
                      aria-current={session.id === activeSession ? 'true' : undefined}
                      aria-label={`${session.title}, ${session.working_directory}`}
                      className={`
                        group flex items-center justify-between p-3 rounded-lg cursor-pointer transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ai-primary
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
                          <div className={`text-xs truncate ${session.id === activeSession ? 'text-white/70' : 'text-terminal-muted'
                            }`}>
                            {session.working_directory}
                          </div>
                        </div>
                      </div>

                      {sessions.length > 1 && (
                        <button
                          type="button"
                          onClick={(e) => handleCloseSession(session.id, e)}
                          className="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 p-1 hover:bg-black/20 rounded transition-all focus:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-white/70"
                          title="Close session"
                          aria-label={`Close ${session.title}`}
                        >
                          <X className="w-3 h-3" />
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            /* File Explorer */
            <FileExplorer
              currentPath={currentExplorerPath}
              activeSessionId={activeSession || undefined}
              onPathChange={(newPath) => {
                setCurrentExplorerPath(newPath);
                // Note: FileExplorer already handles terminal directory change
              }}
              onFileSelect={() => {
                // Here you could implement additional file opening functionality
                // For example, opening files in an editor or showing file contents
              }}
            />
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-terminal-border space-y-1">
          <button
            type="button"
            onClick={async () => {
              try {
                const path = await selectWorkspace();
                if (path) {
                  setCurrentExplorerPath(path);
                  setActiveTab('explorer');
                }
              } catch (error) {
                const detail = error instanceof Error ? error.message : String(error);
                await message(`The selected folder could not be opened.\n\n${detail}`, {
                  title: 'Workspace Access Failed',
                  kind: 'error',
                });
              }
            }}
            disabled={!activeSession}
            className="w-full flex items-center space-x-2 p-2 text-terminal-muted hover:text-terminal-text hover:bg-terminal-border rounded transition-colors focus-ring disabled:opacity-40 disabled:cursor-not-allowed"
            title="Choose a folder the sandboxed terminal can access"
          >
            <FolderOpen className="w-4 h-4" />
            <span className="text-sm">Open Workspace</span>
          </button>
          <button
            type="button"
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
