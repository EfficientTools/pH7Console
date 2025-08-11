import React, { useState, useEffect, useRef } from 'react';
import { useTerminalStore } from '../store/terminalStore';
import { useAIStore } from '../store/aiStore';
import { useHotkeys } from 'react-hotkeys-hook';
import { Terminal as TerminalIcon, Zap, History } from 'lucide-react';

export const Terminal: React.FC = () => {
  const [input, setInput] = useState('');
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [completions, setCompletions] = useState<string[]>([]);
  const [selectedCompletion, setSelectedCompletion] = useState(0);
  const [showHistory, setShowHistory] = useState(false);
  const [hoveredCommand, setHoveredCommand] = useState<string | null>(null);
  const [commandExplanation, setCommandExplanation] = useState<string | null>(null);
  
  const inputRef = useRef<HTMLInputElement>(null);
  const terminalRef = useRef<HTMLDivElement>(null);
  
  const {
    activeSession,
    commandHistory,
    executeCommand,
    isExecuting,
  } = useTerminalStore();
  
  const {
    isModelLoaded,
    getCompletions,
    getSuggestions,
    explainCommand,
  } = useAIStore();

  // Auto-focus input
  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, []);

  // Scroll to bottom when new commands are added
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [commandHistory]);

  // Get smart completions as user types
  useEffect(() => {
    const getSmartCompletions = async () => {
      if (input.trim() && activeSession && isModelLoaded) {
        const completions = await getCompletions(input, activeSession);
        setCompletions(completions);
        setShowSuggestions(completions.length > 0);
        setSelectedCompletion(0);
      } else {
        setShowSuggestions(false);
      }
    };

    const debounceTimer = setTimeout(getSmartCompletions, 300);
    return () => clearTimeout(debounceTimer);
  }, [input, activeSession, isModelLoaded, getCompletions]);

  // Keyboard shortcuts
  useHotkeys('enter', (e) => {
    e.preventDefault();
    handleExecuteCommand();
  }, { enableOnFormTags: true });

  useHotkeys('tab', (e) => {
    if (showSuggestions && completions.length > 0) {
      e.preventDefault();
      setInput(completions[selectedCompletion]);
      setShowSuggestions(false);
    }
  }, { enableOnFormTags: true });

  useHotkeys('arrowdown', (e) => {
    if (showSuggestions) {
      e.preventDefault();
      setSelectedCompletion((prev) => 
        prev < completions.length - 1 ? prev + 1 : 0
      );
    }
  }, { enableOnFormTags: true });

  useHotkeys('arrowup', (e) => {
    if (showSuggestions) {
      e.preventDefault();
      setSelectedCompletion((prev) => 
        prev > 0 ? prev - 1 : completions.length - 1
      );
    }
  }, { enableOnFormTags: true });

  useHotkeys('escape', () => {
    setShowSuggestions(false);
    setShowHistory(false);
    setCommandExplanation(null);
  }, { enableOnFormTags: true });

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
    
    await executeCommand(input);
    setInput('');
    setShowSuggestions(false);
    
    // Get AI suggestions for next command
    if (isModelLoaded && activeSession) {
      await getSuggestions(`Last command: ${input}`);
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setInput(e.target.value);
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
          onClick={() => setShowHistory(!showHistory)}
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
              <span className="text-terminal-muted">$</span>
              <span 
                className="text-terminal-text cursor-pointer hover:text-ai-primary transition-colors"
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
              <div className="command-output pl-4 border-l-2 border-terminal-border">
                <pre className="whitespace-pre-wrap">{execution.output}</pre>
              </div>
            )}
            
            {/* Exit Code */}
            {execution.exit_code !== undefined && execution.exit_code !== 0 && (
              <div className="text-terminal-error text-xs pl-4">
                Exit code: {execution.exit_code}
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

      {/* History Panel */}
      {showHistory && (
        <div className="border-t border-terminal-border bg-terminal-bg/50 backdrop-blur">
          <div className="p-4">
            <div className="flex items-center space-x-2 mb-3">
              <History className="w-4 h-4 text-ai-primary" />
              <span className="text-sm font-medium text-ai-primary">Command History</span>
              <span className="text-xs text-terminal-muted">
                ({commandHistory.length} commands)
              </span>
            </div>
            <div className="space-y-1 max-h-32 overflow-y-auto">
              {commandHistory.slice(-10).map((execution) => (
                <div
                  key={execution.id}
                  className="flex items-center justify-between p-2 rounded hover:bg-terminal-border cursor-pointer group"
                  onClick={() => {
                    setInput(execution.command);
                    setShowHistory(false);
                    inputRef.current?.focus();
                  }}
                >
                  <span className="text-sm font-mono text-terminal-text group-hover:text-ai-primary">
                    {execution.command}
                  </span>
                  <div className="flex items-center space-x-2 text-xs text-terminal-muted">
                    {execution.exit_code === 0 ? (
                      <span className="text-green-400">✓</span>
                    ) : (
                      <span className="text-red-400">✗</span>
                    )}
                    <span>{formatTimestamp(execution.timestamp)}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

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
            className="flex-1 terminal-input"
            placeholder={isModelLoaded ? "Type a command or describe what you want to do..." : "Type a command..."}
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
          <div className="flex items-center space-x-4">
            <span>Press Tab for completions</span>
            <span>Enter to execute</span>
            {isModelLoaded && <span>Describe tasks in natural language</span>}
          </div>
        </div>
      </div>
    </div>
  );
};
