import React, { useState, useEffect, useRef, useCallback } from 'react';
import { History, Search, Clock, CheckCircle, XCircle, X, ArrowUp, ArrowDown, Trash2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { CommandExecution } from '../store/terminalStore';
import { useClickOutside } from '../hooks/useClickOutside';

interface HistoryPersistenceStatus {
  encryptedPersistence: boolean;
  mode: 'encrypted' | 'memory_only';
  message: string;
}

interface HistoryModalProps {
  isOpen: boolean;
  onClose: () => void;
  commandHistory: CommandExecution[];
  onSelectCommand: (command: string) => void;
  onClearHistory: () => Promise<void>;
}

export const HistoryModal: React.FC<HistoryModalProps> = ({
  isOpen,
  onClose,
  commandHistory,
  onSelectCommand,
  onClearHistory,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [filteredHistory, setFilteredHistory] = useState<CommandExecution[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [persistenceStatus, setPersistenceStatus] = useState<HistoryPersistenceStatus | null>(null);
  const [isClearing, setIsClearing] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const modalRef = useRef<HTMLDivElement>(null);

  // Close modal when clicking outside
  useClickOutside(modalRef, onClose, isOpen);

  const handleSelectCommand = useCallback((command: string) => {
    onSelectCommand(command);
    onClose();
  }, [onClose, onSelectCommand]);

  const handleClearHistory = useCallback(async () => {
    if (!window.confirm('Clear all completed command history from this Mac? This cannot be undone.')) {
      return;
    }
    setIsClearing(true);
    try {
      await onClearHistory();
      setSearchQuery('');
    } catch (error) {
      console.error('Could not clear encrypted command history:', error);
      window.alert('Command history could not be cleared. No history was hidden from this view.');
    } finally {
      setIsClearing(false);
    }
  }, [onClearHistory]);

  // Search the complete encrypted FTS index rather than only the recent rows
  // already loaded into the WebView. A local fallback keeps memory-only mode
  // useful and makes transient native failures non-blocking.
  useEffect(() => {
    const query = searchQuery.trim();
    if (!isOpen || query === '') {
      setFilteredHistory(commandHistory.slice().reverse()); // Most recent first
      setIsSearching(false);
      setSelectedIndex(0);
      return;
    }

    let cancelled = false;
    setIsSearching(true);
    const timer = window.setTimeout(() => {
      invoke<CommandExecution[]>('search_command_history_records', { query, limit: 100 })
        .then((records) => {
          if (!cancelled) setFilteredHistory(records);
        })
        .catch(() => {
          if (cancelled) return;
          const queryLower = query.toLowerCase();
          setFilteredHistory(
            commandHistory
              .filter(command => command.command.toLowerCase().includes(queryLower))
              .reverse()
          );
        })
        .finally(() => {
          if (!cancelled) setIsSearching(false);
        });
    }, 120);

    setSelectedIndex(0);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [commandHistory, isOpen, searchQuery]);

  // Auto-focus search input when modal opens
  useEffect(() => {
    if (isOpen && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    let cancelled = false;
    invoke<HistoryPersistenceStatus>('get_history_persistence_status')
      .then((status) => {
        if (!cancelled) setPersistenceStatus(status);
      })
      .catch(() => {
        if (!cancelled) setPersistenceStatus(null);
      });
    return () => {
      cancelled = true;
    };
  }, [isOpen]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;

      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex(prev => 
            prev < filteredHistory.length - 1 ? prev + 1 : prev
          );
          break;
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex(prev => prev > 0 ? prev - 1 : prev);
          break;
        case 'Enter':
          e.preventDefault();
          if (filteredHistory[selectedIndex]) {
            handleSelectCommand(filteredHistory[selectedIndex].command);
          }
          break;
        case 'Escape':
          e.preventDefault();
          onClose();
          break;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, filteredHistory, selectedIndex, handleSelectCommand, onClose]);

  // Scroll selected item into view
  useEffect(() => {
    const selectedElement = document.querySelector(`[data-history-index="${selectedIndex}"]`);
    if (selectedElement) {
      selectedElement.scrollIntoView({ 
        behavior: 'smooth', 
        block: 'nearest' 
      });
    }
  }, [selectedIndex]);

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffHours = diffMs / (1000 * 60 * 60);
    const diffDays = diffHours / 24;

    if (diffHours < 1) {
      const diffMinutes = Math.floor(diffMs / (1000 * 60));
      return diffMinutes < 1 ? 'Just now' : `${diffMinutes}m ago`;
    } else if (diffHours < 24) {
      return `${Math.floor(diffHours)}h ago`;
    } else if (diffDays < 7) {
      return `${Math.floor(diffDays)}d ago`;
    } else {
      return date.toLocaleDateString();
    }
  };

  const formatDuration = (durationMs: number) => {
    if (durationMs < 1000) {
      return `${durationMs}ms`;
    } else {
      return `${(durationMs / 1000).toFixed(1)}s`;
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-3 backdrop-blur-sm sm:p-6">
      <div 
        ref={modalRef}
        className="flex max-h-[calc(100vh-1.5rem)] min-h-0 w-full max-w-4xl flex-col overflow-hidden rounded-xl border border-terminal-border bg-terminal-bg shadow-2xl sm:max-h-[80vh]"
        role="dialog"
        aria-modal="true"
        aria-labelledby="history-title"
      >
        {/* Header */}
        <div className="flex items-start justify-between gap-3 border-b border-terminal-border p-4">
          <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
            <History className="h-5 w-5 flex-none text-ai-primary" />
            <h2 id="history-title" className="text-lg font-semibold text-terminal-text">Command History</h2>
            <span className="break-words text-sm text-terminal-muted">
              {searchQuery.trim()
                ? `(${isSearching ? 'Searching…' : `${filteredHistory.length} results`})`
                : `(${commandHistory.length} recent commands)`}
            </span>
            {persistenceStatus && (
              <span
                className={`rounded-full border px-2 py-0.5 text-xs ${
                  persistenceStatus.encryptedPersistence
                    ? 'border-green-400/30 bg-green-400/10 text-green-300'
                    : 'border-amber-400/30 bg-amber-400/10 text-amber-300'
                }`}
                title={persistenceStatus.message}
              >
                {persistenceStatus.message}
              </span>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            className="flex-none rounded p-1 transition-colors hover:bg-terminal-border"
            aria-label="Close command history"
          >
            <X className="w-5 h-5 text-terminal-muted" />
          </button>
        </div>

        {/* Search */}
        <div className="p-4 border-b border-terminal-border">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-terminal-muted" />
            <input
              ref={searchInputRef}
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search command history... (use ↑↓ to navigate, Enter to select)"
              className="w-full pl-10 pr-4 py-2 bg-terminal-bg border border-terminal-border rounded focus:border-ai-primary focus:ring-1 focus:ring-ai-primary transition-colors text-terminal-text placeholder-terminal-muted"
              aria-label="Search command history"
            />
          </div>
        </div>

        {/* History List */}
        <div className="flex-1 overflow-hidden">
          {filteredHistory.length === 0 ? (
            <div className="flex items-center justify-center h-32 text-terminal-muted">
              <div className="text-center">
                <History className="w-8 h-8 mx-auto mb-2 opacity-50" />
                <p>
                  {searchQuery ? 'No commands found matching your search' : 'No command history yet'}
                </p>
              </div>
            </div>
          ) : (
            <div className="overflow-y-auto max-h-full" role="listbox" aria-label="Command history results">
              {filteredHistory.map((execution, index) => (
                <div
                  key={execution.id}
                  data-history-index={index}
                  className={`border-b border-terminal-border last:border-b-0 transition-colors cursor-pointer ${
                    index === selectedIndex 
                      ? 'bg-ai-primary/10 border-ai-primary/20' 
                      : 'hover:bg-terminal-border/50'
                  }`}
                  onClick={() => handleSelectCommand(execution.command)}
                  role="option"
                  aria-selected={index === selectedIndex}
                >
                  <div className="p-4">
                    {/* Command and Status */}
                    <div className="flex flex-col items-start gap-3 sm:flex-row sm:justify-between">
                      <div className="min-w-0 flex-1 sm:mr-4">
                        <div className="flex items-center space-x-2 mb-1">
                          {execution.exit_code === 0 ? (
                            <CheckCircle className="w-4 h-4 text-green-400 flex-shrink-0" />
                          ) : (
                            <XCircle className="w-4 h-4 text-red-400 flex-shrink-0" />
                          )}
                          <code className={`font-mono text-sm break-all ${
                            execution.exit_code === 0 ? 'text-terminal-text' : 'text-red-300'
                          }`}>
                            {execution.command}
                          </code>
                        </div>
                        
                        {/* Output preview (if exists and not too long) */}
                        {execution.output && execution.output.length < 200 && (
                          <div className="mt-2 text-xs text-terminal-muted bg-terminal-bg/50 rounded p-2 border border-terminal-border/30">
                            <pre className="whitespace-pre-wrap truncate">
                              {execution.output.slice(0, 150)}
                              {execution.output.length > 150 && '...'}
                            </pre>
                          </div>
                        )}
                      </div>
                      
                      {/* Metadata */}
                      <div className="flex flex-none flex-row flex-wrap items-center gap-2 text-xs text-terminal-muted sm:flex-col sm:items-end sm:space-y-1">
                        <div className="flex items-center space-x-1">
                          <Clock className="w-3 h-3" />
                          <span>{formatTimestamp(execution.timestamp)}</span>
                        </div>
                        <div className="flex items-center space-x-2">
                          <span className={`px-2 py-1 rounded text-xs font-medium ${
                            execution.exit_code === 0 
                              ? 'bg-green-400/20 text-green-300' 
                              : 'bg-red-400/20 text-red-300'
                          }`}>
                            {execution.exit_code === 0 ? 'Success' : `Exit ${execution.exit_code}`}
                          </span>
                          <span className="text-terminal-muted">
                            {formatDuration(execution.duration_ms)}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-terminal-border bg-terminal-bg/50">
          <div className="flex flex-col gap-3 text-xs text-terminal-muted sm:flex-row sm:items-center sm:justify-between">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
              <div className="flex items-center space-x-1">
                <ArrowUp className="w-3 h-3" />
                <ArrowDown className="w-3 h-3" />
                <span>Navigate</span>
              </div>
              <div className="flex items-center space-x-1">
                <kbd className="px-1 py-0.5 bg-terminal-border rounded text-xs">Enter</kbd>
                <span>Select</span>
              </div>
              <div className="flex items-center space-x-1">
                <kbd className="px-1 py-0.5 bg-terminal-border rounded text-xs">Esc</kbd>
                <span>Close</span>
              </div>
            </div>
            <div className="flex flex-wrap items-center justify-between gap-3 sm:justify-end">
              <span>
                {filteredHistory.length === 0 ? 0 : selectedIndex + 1} of {filteredHistory.length}
              </span>
              <button
                type="button"
                onClick={() => void handleClearHistory()}
                disabled={isClearing || commandHistory.length === 0}
                className="inline-flex items-center gap-1 rounded border border-red-400/30 px-2 py-1 text-red-300 transition-colors hover:bg-red-400/10 disabled:cursor-not-allowed disabled:opacity-40"
              >
                <Trash2 className="h-3 w-3" />
                {isClearing ? 'Clearing…' : 'Clear all'}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
