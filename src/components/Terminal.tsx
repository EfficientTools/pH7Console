import React, { useState, useEffect, useRef } from 'react';
import { useTerminalStore } from '../store/terminalStore';
import { useAIStore } from '../store/aiStore';
import { useHotkeys } from 'react-hotkeys-hook';
import { Terminal as TerminalIcon, Zap, History } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { HistoryModal } from './HistoryModal';

export const Terminal: React.FC = () => {
  const [input, setInput] = useState('');
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [completions, setCompletions] = useState<string[]>([]);
  const [selectedCompletion, setSelectedCompletion] = useState(0);
  const [showHistoryModal, setShowHistoryModal] = useState(false);
  const [hoveredCommand, setHoveredCommand] = useState<string | null>(null);
  const [commandExplanation, setCommandExplanation] = useState<string | null>(null);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [historyCommands, setHistoryCommands] = useState<string[]>([]);
  const [originalInput, setOriginalInput] = useState('');
  
  const inputRef = useRef<HTMLInputElement>(null);
  const terminalRef = useRef<HTMLDivElement>(null);
  
  const {
    activeSession,
    commandHistory,
    executeCommand,
    clearHistory,
    isExecuting,
  } = useTerminalStore();
  
  const {
    isModelLoaded,
    getCompletions,
    getSuggestions,
    explainCommand,
    translateNaturalLanguage,
    addSuggestion,
  } = useAIStore();

  // Load command history for navigation
  useEffect(() => {
    const loadCommandHistory = async () => {
      if (activeSession) {
        try {
          const history = await invoke<string[]>('get_command_history_for_navigation', {
            sessionId: activeSession,
          });
          // Backend already returns in reverse chronological order (most recent first)
          setHistoryCommands(history);
        } catch (error) {
          console.error('Failed to load command history:', error);
        }
      }
    };

    loadCommandHistory();
    // Reload history whenever command history changes
  }, [activeSession, commandHistory.length]);

  // Auto-focus input when component mounts and when app becomes active
  useEffect(() => {
    const focusInput = () => {
      if (inputRef.current) {
        inputRef.current.focus();
      }
    };

    // Focus immediately
    focusInput();

    // Focus when the window becomes active
    const handleWindowFocus = () => {
      setTimeout(focusInput, 100); // Small delay to ensure DOM is ready
    };

    // Focus when clicking anywhere in the terminal area
    const handleTerminalClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      // Only focus if not clicking on a button or other interactive element
      if (!target.closest('button') && !target.closest('input') && !target.closest('textarea')) {
        focusInput();
      }
    };

    window.addEventListener('focus', handleWindowFocus);
    document.addEventListener('click', handleTerminalClick);

    return () => {
      window.removeEventListener('focus', handleWindowFocus);
      document.removeEventListener('click', handleTerminalClick);
    };
  }, []);

  // Re-focus when activeSession changes
  useEffect(() => {
    if (inputRef.current && activeSession) {
      setTimeout(() => {
        inputRef.current?.focus();
      }, 100);
    }
  }, [activeSession]);

  // Scroll to bottom when new commands are added
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [commandHistory]);

  // Enhanced natural language detection and processing
  const detectNaturalLanguage = (input: string): boolean => {
    const naturalLanguageIndicators = [
      'show me', 'find all', 'list all', 'how do i', 'what is', 'where are',
      'can you', 'please', 'i want to', 'i need to', 'help me',
      'search for', 'look for', 'give me', 'tell me', 'count the',
      'delete all', 'remove all', 'copy all', 'move all', 'create a',
      'make a', 'install the', 'update the', 'check the', 'run the'
    ];
    
    const inputLower = input.toLowerCase();
    return naturalLanguageIndicators.some(indicator => inputLower.includes(indicator)) ||
           (input.includes(' ') && !input.startsWith('/') && !input.startsWith('~') && 
            Boolean(inputLower.match(/^[a-z\s]+$/))); // Contains only letters and spaces
  };

  const handleNaturalLanguageDetection = async (input: string) => {
    if (detectNaturalLanguage(input) && isModelLoaded && activeSession) {
      try {
        const context = commandHistory.slice(-3).map(cmd => cmd.command).join('; ');
        const response = await translateNaturalLanguage(input, context);
        
        if (response.text && !response.text.startsWith('#')) {
          // Add natural language suggestion
          const nlSuggestion = {
            id: Date.now().toString(),
            type: 'command' as const,
            content: response.text,
            confidence: response.confidence,
            timestamp: Date.now(),
          };
          
          addSuggestion(nlSuggestion);
          
          // Auto-suggest this as a completion
          setCompletions([response.text]);
          setShowSuggestions(true);
          setSelectedCompletion(0);
        }
      } catch (error) {
        console.error('Natural language detection failed:', error);
      }
    }
  };

  // Get smart completions as user types (with natural language detection)
  useEffect(() => {
    const getSmartCompletions = async () => {
      if (input.trim() && activeSession && isModelLoaded && historyIndex === -1) {
        // First try natural language detection
        if (detectNaturalLanguage(input)) {
          await handleNaturalLanguageDetection(input);
        } else {
          // Regular command completions
          const completions = await getCompletions(input, activeSession);
          setCompletions(completions);
          setShowSuggestions(completions.length > 0);
          setSelectedCompletion(0);
        }
      } else {
        setShowSuggestions(false);
      }
    };

    const debounceTimer = setTimeout(getSmartCompletions, 500); // Slightly longer delay for natural language
    return () => clearTimeout(debounceTimer);
  }, [input, activeSession, isModelLoaded, getCompletions, historyIndex]);

  // Tab completion for file paths
  const handleTabCompletion = async () => {
    if (showSuggestions && completions.length > 0) {
      setInput(completions[selectedCompletion]);
      setShowSuggestions(false);
      return;
    }

    if (!activeSession) return;

    try {
      // Extract the last word (potential path) from input
      const parts = input.split(' ');
      const lastPart = parts[parts.length - 1];
      
      const pathCompletions = await invoke<string[]>('get_path_completions', {
        sessionId: activeSession,
        partialPath: lastPart,
      });

      if (pathCompletions.length === 1) {
        // Single completion - auto-complete
        parts[parts.length - 1] = pathCompletions[0];
        setInput(parts.join(' '));
      } else if (pathCompletions.length > 1) {
        // Multiple completions - show them
        setCompletions(pathCompletions.map((completion: string) => {
          const newParts = [...parts];
          newParts[newParts.length - 1] = completion;
          return newParts.join(' ');
        }));
        setShowSuggestions(true);
        setSelectedCompletion(0);
      }
    } catch (error) {
      console.error('Tab completion failed:', error);
    }
  };

  // Arrow key navigation through command history
  const navigateHistory = (direction: 'up' | 'down') => {
    if (historyCommands.length === 0) return;

    if (direction === 'up') {
      if (historyIndex === -1) {
        // First time pressing up - save current input and go to latest command
        setOriginalInput(input);
        setHistoryIndex(0);
        setInput(historyCommands[0]);
      } else if (historyIndex < historyCommands.length - 1) {
        // Go to older command
        const newIndex = historyIndex + 1;
        setHistoryIndex(newIndex);
        setInput(historyCommands[newIndex]);
      }
    } else {
      // Down
      if (historyIndex > 0) {
        // Go to newer command
        const newIndex = historyIndex - 1;
        setHistoryIndex(newIndex);
        setInput(historyCommands[newIndex]);
      } else if (historyIndex === 0) {
        // Back to original input
        setHistoryIndex(-1);
        setInput(originalInput);
      }
    }
  };

  // Handle keyboard events directly on the input
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    const inputElement = e.currentTarget;
    const cursorPos = inputElement.selectionStart || 0;
    const inputValue = inputElement.value;

    // Handle modifier key combinations first
    if (e.ctrlKey || e.metaKey) {
      switch (e.key) {
        case 'u': // Ctrl+U: Delete to start of line (ASCII 0x15)
          e.preventDefault();
          const afterCursor = inputValue.substring(cursorPos);
          setInput(afterCursor);
          setTimeout(() => inputElement.setSelectionRange(0, 0), 0);
          setHistoryIndex(-1);
          setOriginalInput('');
          return;
        
        case 'k': // Ctrl+K: Delete to end of line (ASCII 0x0B)
          e.preventDefault();
          const beforeCursor = inputValue.substring(0, cursorPos);
          setInput(beforeCursor);
          setTimeout(() => inputElement.setSelectionRange(cursorPos, cursorPos), 0);
          return;
        
        case 'w': // Ctrl+W: Delete previous word (ASCII 0x17)
          e.preventDefault();
          const beforeCursorW = inputValue.substring(0, cursorPos);
          const afterCursorW = inputValue.substring(cursorPos);
          
          // Find the start of the current word by looking backwards
          const wordMatch = beforeCursorW.match(/\S*\s*$/);
          if (wordMatch) {
            const newBeforeCursor = beforeCursorW.substring(0, beforeCursorW.length - wordMatch[0].length);
            const newValue = newBeforeCursor + afterCursorW;
            setInput(newValue);
            setTimeout(() => inputElement.setSelectionRange(newBeforeCursor.length, newBeforeCursor.length), 0);
          }
          return;
        
        case 'l': // Ctrl+L: Clear screen (Form Feed - ASCII 0x0C)
          e.preventDefault();
          clearHistory();
          return;
        
        case 'd': // Ctrl+D: Exit shell (EOT - ASCII 0x04)
          e.preventDefault();
          if (inputValue.trim() === '') {
            if (activeSession) {
              executeCommand('exit');
            }
          }
          return;
        
        case 'c': // Ctrl+C: Cancel current input (ETX - ASCII 0x03)
          e.preventDefault();
          setInput('');
          setHistoryIndex(-1);
          setOriginalInput('');
          setShowSuggestions(false);
          return;
        
        case 'h': // Ctrl+H: Backspace (BS - ASCII 0x08)
          e.preventDefault();
          if (cursorPos > 0) {
            const newValue = inputValue.substring(0, cursorPos - 1) + inputValue.substring(cursorPos);
            setInput(newValue);
            setTimeout(() => inputElement.setSelectionRange(cursorPos - 1, cursorPos - 1), 0);
          }
          return;
        
        case 'r': // Ctrl+R: Reverse history search
          e.preventDefault();
          setShowHistoryModal(true);
          return;
      }
    }

    // Handle Option/Alt key combinations for word movement
    if (e.altKey) {
      switch (e.key) {
        case 'ArrowLeft': // Alt+Left: Move cursor back by word
          e.preventDefault();
          const beforeCursorLeft = inputValue.substring(0, cursorPos);
          // Find the start of the previous word
          const leftMatch = beforeCursorLeft.match(/\S+\s*$/);
          if (leftMatch) {
            const targetPos = beforeCursorLeft.length - leftMatch[0].length;
            setTimeout(() => inputElement.setSelectionRange(targetPos, targetPos), 0);
          } else {
            setTimeout(() => inputElement.setSelectionRange(0, 0), 0);
          }
          return;
        
        case 'ArrowRight': // Alt+Right: Move cursor forward by word
          e.preventDefault();
          const afterCursorRight = inputValue.substring(cursorPos);
          // Find the end of the next word
          const rightMatch = afterCursorRight.match(/^\s*\S+/);
          if (rightMatch) {
            const targetPos = cursorPos + rightMatch[0].length;
            setTimeout(() => inputElement.setSelectionRange(targetPos, targetPos), 0);
          } else {
            setTimeout(() => inputElement.setSelectionRange(inputValue.length, inputValue.length), 0);
          }
          return;
        
        case 'Backspace': // Alt+Backspace: Delete previous word
          e.preventDefault();
          const beforeCursorAltBS = inputValue.substring(0, cursorPos);
          const afterCursorAltBS = inputValue.substring(cursorPos);
          
          const wordMatchAlt = beforeCursorAltBS.match(/\S+\s*$/);
          if (wordMatchAlt) {
            const newBeforeCursor = beforeCursorAltBS.substring(0, beforeCursorAltBS.length - wordMatchAlt[0].length);
            const newValue = newBeforeCursor + afterCursorAltBS;
            setInput(newValue);
            setTimeout(() => inputElement.setSelectionRange(newBeforeCursor.length, newBeforeCursor.length), 0);
          }
          return;
      }
    }

    // Handle regular keys
    switch (e.key) {
      case 'Enter': // ASCII 0x0A (LF) or 0x0D (CR)
        e.preventDefault();
        handleExecuteCommand();
        break;
      
      case 'Tab':
        e.preventDefault();
        handleTabCompletion();
        break;
      
      case 'ArrowUp':
        e.preventDefault();
        if (showSuggestions) {
          setSelectedCompletion((prev) => 
            prev > 0 ? prev - 1 : completions.length - 1
          );
        } else {
          navigateHistory('up');
        }
        break;
      
      case 'ArrowDown':
        e.preventDefault();
        if (showSuggestions) {
          setSelectedCompletion((prev) => 
            prev < completions.length - 1 ? prev + 1 : 0
          );
        } else {
          navigateHistory('down');
        }
        break;
      
      case 'ArrowLeft':
      case 'ArrowRight':
        // Allow normal cursor movement (don't prevent default)
        break;
      
      case 'Escape':
        e.preventDefault();
        setShowSuggestions(false);
        setShowHistoryModal(false);
        setCommandExplanation(null);
        if (historyIndex !== -1) {
          setHistoryIndex(-1);
          setInput(originalInput);
        }
        break;
    }
  };

  // Global keyboard shortcuts (not input-specific)
  useHotkeys('escape', () => {
    if (showHistoryModal) {
      setShowHistoryModal(false);
    }
  }, { enableOnFormTags: false });

  // Handle command explanation on hover
  const handleCommandHover = async (command: string) => {
    if (hoveredCommand === command || !isModelLoaded) return;
    
    setHoveredCommand(command);
    try {
      const explanation = await explainCommand(command);
      setCommandExplanation(explanation.text);
    } catch (error) {
      console.error('Failed to get explanation:', error);
    }
  };

  const handleCommandLeave = () => {
    setHoveredCommand(null);
    setCommandExplanation(null);
  };

  const handleExecuteCommand = async () => {
    if (!input.trim() || isExecuting) return;
    
    // Handle clear command specially - clear the display instead of executing it
    if (input.trim().toLowerCase() === 'clear') {
      clearHistory();
      setInput('');
      setShowSuggestions(false);
      setHistoryIndex(-1);
      setOriginalInput('');
      return;
    }
    
    await executeCommand(input);
    setInput('');
    setShowSuggestions(false);
    setHistoryIndex(-1);
    setOriginalInput('');
    
    // Get AI suggestions for next command
    if (isModelLoaded && activeSession) {
      await getSuggestions(`Last command: ${input}`);
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setInput(newValue);
    
    // Reset history navigation when user types
    if (historyIndex !== -1) {
      setHistoryIndex(-1);
      setOriginalInput('');
    }
  };

  const getCurrentPath = () => {
    // Extract current path from last command or default to ~
    const lastCommand = commandHistory[commandHistory.length - 1];
    if (lastCommand?.command === 'pwd') {
      return lastCommand.output || '~';
    }
    return '~';
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  if (!activeSession) {
    return (
      <div className="h-full flex items-center justify-center text-terminal-muted">
        <div className="text-center">
          <TerminalIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
          <p>No active terminal session</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-terminal-bg">
      {/* Header with History Toggle */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-terminal-border">
        <div className="flex items-center space-x-2">
          <TerminalIcon className="w-5 h-5 text-ai-primary" />
          <span className="text-sm font-medium text-terminal-text">
            Session: {activeSession}
          </span>
        </div>
        <button
          onClick={() => setShowHistoryModal(true)}
          className="flex items-center space-x-1 px-3 py-1 rounded hover:bg-terminal-border transition-colors"
        >
          <History className="w-4 h-4 text-terminal-muted" />
          <span className="text-xs text-terminal-muted">History</span>
        </button>
      </div>

      {/* Command Explanation Tooltip */}
      {commandExplanation && hoveredCommand && (
        <div className="mx-4 mt-2 p-3 bg-ai-primary/10 border border-ai-primary/20 rounded">
          <div className="flex items-center space-x-2 mb-1">
            <Zap className="w-4 h-4 text-ai-primary" />
            <span className="text-xs font-medium text-ai-primary">Command Explanation</span>
          </div>
          <p className="text-sm text-terminal-text">{commandExplanation}</p>
        </div>
      )}

      {/* Terminal Output */}
      <div 
        ref={terminalRef}
        className="flex-1 overflow-y-auto p-4 space-y-2 font-mono text-sm"
      >
        {commandHistory.map((execution) => (
          <div key={execution.id} className="space-y-1">
            {/* Command */}
            <div className="flex items-center space-x-2">
              <span className="text-ai-primary font-medium">
                {getCurrentPath()}
              </span>
              <span className={`${
                execution.exit_code !== undefined && execution.exit_code !== 0 
                  ? 'text-red-400' 
                  : 'text-terminal-muted'
              }`}>$</span>
              <span 
                className={`cursor-pointer hover:text-ai-primary transition-colors ${
                  execution.exit_code !== undefined && execution.exit_code !== 0
                    ? 'text-red-200'
                    : 'text-terminal-text'
                }`}
                onMouseEnter={() => handleCommandHover(execution.command)}
                onMouseLeave={handleCommandLeave}
                title={isModelLoaded ? "Hover for AI explanation" : ""}
              >
                {execution.command}
              </span>
              <span className="text-terminal-muted text-xs ml-auto">
                {formatTimestamp(execution.timestamp)}
              </span>
            </div>
            
            {/* Output */}
            {execution.output && (
              <div className={`command-output pl-4 border-l-2 ${
                execution.exit_code !== undefined && execution.exit_code !== 0 
                  ? 'border-red-400/50 bg-red-400/5' 
                  : 'border-terminal-border'
              }`}>
                <pre className={`whitespace-pre-wrap ${
                  execution.exit_code !== undefined && execution.exit_code !== 0
                    ? 'text-red-200'
                    : ''
                }`}>{execution.output}</pre>
              </div>
            )}
            
            {/* Success/Error Indicator - Modern Style */}
            {execution.exit_code !== undefined && (
              <div className={`flex items-center space-x-2 text-xs pl-4 mt-1 ${
                execution.exit_code === 0 
                  ? 'text-green-400' 
                  : 'text-red-400'
              }`}>
                {execution.exit_code === 0 ? (
                  <>
                    <span className="flex items-center space-x-1">
                      <span className="w-2 h-2 bg-green-400 rounded-full"></span>
                      <span>Success</span>
                    </span>
                    <span className="text-terminal-muted">
                      ({execution.duration_ms}ms)
                    </span>
                  </>
                ) : (
                  <>
                    <span className="flex items-center space-x-1">
                      <span className="w-2 h-2 bg-red-400 rounded-full"></span>
                      <span>Failed</span>
                    </span>
                    <span className="text-terminal-muted">
                      (exit {execution.exit_code} • {execution.duration_ms}ms)
                    </span>
                  </>
                )}
              </div>
            )}
          </div>
        ))}
        
        {/* Loading indicator */}
        {isExecuting && (
          <div className="flex items-center space-x-2 text-terminal-muted">
            <div className="w-2 h-2 bg-ai-primary rounded-full animate-bounce"></div>
            <span>Executing...</span>
          </div>
        )}
      </div>

      {/* History Modal */}
      <HistoryModal
        isOpen={showHistoryModal}
        onClose={() => setShowHistoryModal(false)}
        commandHistory={commandHistory}
        onSelectCommand={(command) => {
          setInput(command);
          inputRef.current?.focus();
        }}
      />

      {/* Input Area */}
      <div className="border-t border-terminal-border p-4">
        {/* AI Suggestions */}
        {showSuggestions && completions.length > 0 && (
          <div className="mb-3 ai-suggestion">
            <div className="flex items-center space-x-2 mb-2">
              <Zap className="w-4 h-4 text-ai-primary" />
              <span className="text-xs font-medium text-ai-primary">Smart Completions</span>
            </div>
            <div className="space-y-1">
              {completions.map((completion, index) => (
                <div
                  key={completion}
                  className={`px-3 py-1 rounded cursor-pointer text-sm ${
                    index === selectedCompletion
                      ? 'bg-ai-primary text-white'
                      : 'text-terminal-text hover:bg-terminal-border'
                  }`}
                  onClick={() => {
                    setInput(completion);
                    setShowSuggestions(false);
                  }}
                >
                  {completion}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Command Input */}
        <div className="flex items-center space-x-2">
          <span className="text-ai-primary font-medium font-mono">
            {getCurrentPath()}
          </span>
          <span className="text-terminal-muted font-mono">$</span>
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            className="flex-1 terminal-input"
            placeholder={
              historyCommands.length > 0 
                ? "Type a command or natural language... (Use ↑↓ for history, try 'show me large files')" 
                : isModelLoaded 
                  ? "Type a command or describe what you want to do in plain English..." 
                  : "Type a command..."
            }
            disabled={isExecuting}
          />
          
          {isModelLoaded && (
            <div className="flex items-center space-x-1">
              <Zap className="w-4 h-4 text-ai-primary" />
              <span className="text-xs text-ai-primary">AI</span>
            </div>
          )}
        </div>
        
        {/* Quick Tips */}
        <div className="mt-2 text-xs text-terminal-muted">
          <div className="flex items-center space-x-4 flex-wrap">
            <span>Tab: autocomplete</span>
            <span>↑↓: history</span>
            <span>⌃R: search history</span>
            <span>⌃U: clear line</span>
            <span>⌃K: clear to end</span>
            <span>⌃W: delete word</span>
            <span>⌃L: clear screen</span>
            <span>⌃C: cancel</span>
            <span>⌃D: exit</span>
            <span>⌥←→: word jump</span>
            <span>⌃H: backspace</span>
            {isModelLoaded && <span>• AI-powered</span>}
          </div>
        </div>
      </div>
    </div>
  );
};
