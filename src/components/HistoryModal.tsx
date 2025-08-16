import React, { useState, useEffect, useRef } from 'react';
import { History, Search, Clock, CheckCircle, XCircle, X, ArrowUp, ArrowDown } from 'lucide-react';
import { CommandExecution } from '../store/terminalStore';
import { useClickOutside } from '../hooks/useClickOutside';

interface HistoryModalProps {
  isOpen: boolean;
  onClose: () => void;
  commandHistory: CommandExecution[];
  onSelectCommand: (command: string) => void;
}

export const HistoryModal: React.FC<HistoryModalProps> = ({
  isOpen,
  onClose,
  commandHistory,
  onSelectCommand,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [filteredHistory, setFilteredHistory] = useState<CommandExecution[]>([]);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const modalRef = useRef<HTMLDivElement>(null);

  // Close modal when clicking outside
  useClickOutside(modalRef, onClose, isOpen);

  // Filter history based on search query
  useEffect(() => {
    if (searchQuery.trim() === '') {
      setFilteredHistory(commandHistory.slice().reverse()); // Most recent first
    } else {
      const filtered = commandHistory
        .filter(cmd => 
          cmd.command.toLowerCase().includes(searchQuery.toLowerCase())
        )
        .reverse(); // Most recent first
      setFilteredHistory(filtered);
    }
    setSelectedIndex(0); // Reset selection when search changes
  }, [searchQuery, commandHistory]);

  // Auto-focus search input when modal opens
  useEffect(() => {
    if (isOpen && searchInputRef.current) {
      searchInputRef.current.focus();
    }
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
  }, [isOpen, filteredHistory, selectedIndex]);

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

  const handleSelectCommand = (command: string) => {
    onSelectCommand(command);
    onClose();
  };

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
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div 
        ref={modalRef}
        className="bg-terminal-bg border border-terminal-border rounded-lg shadow-2xl w-full max-w-4xl max-h-[80vh] flex flex-col"
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-terminal-border">
          <div className="flex items-center space-x-3">
            <History className="w-5 h-5 text-ai-primary" />
            <h2 className="text-lg font-semibold text-terminal-text">Command History</h2>
            <span className="text-sm text-terminal-muted">
              ({filteredHistory.length} of {commandHistory.length} commands)
            </span>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-terminal-border transition-colors"
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
            <div className="overflow-y-auto max-h-full">
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
                >
                  <div className="p-4">
                    {/* Command and Status */}
                    <div className="flex items-start justify-between mb-2">
                      <div className="flex-1 mr-4">
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
                      <div className="flex flex-col items-end space-y-1 text-xs text-terminal-muted">
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
          <div className="flex items-center justify-between text-xs text-terminal-muted">
            <div className="flex items-center space-x-4">
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
            <div>
              {selectedIndex + 1} of {filteredHistory.length}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
