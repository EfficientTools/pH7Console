import React from 'react';
import { useAIStore } from '../store/aiStore';
import { useTerminalStore } from '../store/terminalStore';
import { Brain, Lightbulb, AlertCircle, Zap, MessageSquare, ThumbsUp, ThumbsDown, Copy } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

export const AIPanel: React.FC = () => {
  const { 
    isModelLoaded, 
    suggestions, 
    isProcessing, 
    clearSuggestions,
    translateNaturalLanguage,
    addSuggestion,
    updateFeedback
  } = useAIStore();
  
  const { activeSession, commandHistory } = useTerminalStore();
  const [naturalLanguageInput, setNaturalLanguageInput] = React.useState('');
  const [quickActionLoading, setQuickActionLoading] = React.useState<string | null>(null);

  const handleNaturalLanguageSubmit = async () => {
    if (!naturalLanguageInput.trim() || !activeSession) return;
    
    const context = commandHistory.slice(-3).map(cmd => cmd.command).join('; ');
    
    try {
      const response = await translateNaturalLanguage(naturalLanguageInput, context);
      
      // If we get a valid command, add it to suggestions
      if (response.text && !response.text.startsWith('#')) {
        addSuggestion({
          id: Date.now().toString(),
          type: 'command',
          content: `💡 Natural Language → Command: ${response.text}`,
          confidence: response.confidence,
          timestamp: Date.now()
        });
        
        // Add explanation if available
        if (response.reasoning) {
          addSuggestion({
            id: (Date.now() + 1).toString(),
            type: 'explanation',
            content: response.reasoning,
            confidence: response.confidence,
            timestamp: Date.now()
          });
        }
      } else {
        // Add the response as a suggestion even if it's not a command
        addSuggestion({
          id: Date.now().toString(),
          type: 'explanation',
          content: response.text,
          confidence: response.confidence,
          timestamp: Date.now()
        });
      }
    } catch (error) {
      console.error('Natural language translation failed:', error);
      addSuggestion({
        id: Date.now().toString(),
        type: 'error',
        content: 'Failed to translate natural language. The AI model might not be ready yet.',
        confidence: 0.1,
        timestamp: Date.now()
      });
    }
    
    setNaturalLanguageInput('');
  };

  const handleQuickAction = async (action: 'explain' | 'fix' | 'optimize' | 'analyze') => {
    if (!activeSession || commandHistory.length === 0) return;

    const lastCommand = commandHistory[commandHistory.length - 1];
    if (!lastCommand) return;

    setQuickActionLoading(action);

    try {
      let response: any = null;
      
      switch (action) {
        case 'explain':
          response = await invoke('ai_explain_command', {
            command: lastCommand.command
          });
          addSuggestion({
            id: Date.now().toString(),
            type: 'explanation',
            content: response.text || response,
            confidence: response.confidence || 0.9,
            timestamp: Date.now()
          });
          break;
          
        case 'fix':
          response = await invoke('ai_fix_error', {
            command: lastCommand.command,
            error_output: lastCommand.output || '',
            context: commandHistory.slice(-3).map(cmd => cmd.command).join('; ')
          });
          addSuggestion({
            id: Date.now().toString(),
            type: 'fix',
            content: response.text || response,
            confidence: response.confidence || 0.85,
            timestamp: Date.now()
          });
          break;
          
        case 'optimize':
          response = await invoke('ai_suggest_command', {
            context: `Optimize this command: ${lastCommand.command}`,
            intent: 'optimization'
          });
          addSuggestion({
            id: Date.now().toString(),
            type: 'optimization',
            content: response.text || response,
            confidence: response.confidence || 0.8,
            timestamp: Date.now()
          });
          break;
          
        case 'analyze':
          response = await invoke('ai_analyze_output', {
            command: lastCommand.command,
            output: lastCommand.output || ''
          });
          addSuggestion({
            id: Date.now().toString(),
            type: 'analysis',
            content: response.text || response,
            confidence: response.confidence || 0.9,
            timestamp: Date.now()
          });
          break;
      }
    } catch (error) {
      console.error(`Failed to execute ${action} action:`, error);
      
      // More specific error handling
      let errorMessage = `Failed to ${action} command.`;
      if (error instanceof Error) {
        errorMessage += ` Error: ${error.message}`;
      } else if (typeof error === 'string') {
        errorMessage += ` Error: ${error}`;
      }
      
      // Check if it's a model loading issue
      if (!isModelLoaded) {
        errorMessage += ' The AI model may not be loaded yet. Please wait for model initialization to complete.';
      }
      
      addSuggestion({
        id: Date.now().toString(),
        type: 'error',
        content: errorMessage,
        confidence: 0.5,
        timestamp: Date.now()
      });
    } finally {
      setQuickActionLoading(null);
    }
  };

  return (
    <div className="h-full bg-terminal-surface flex flex-col min-h-0">
      {/* Header */}
      <div className="p-4 border-b border-terminal-border flex-shrink-0">
        <div className="flex items-center space-x-2">
          <Brain className="w-5 h-5 text-ai-primary" />
          <h2 className="font-semibold text-terminal-text">AI Assistant</h2>
          {isModelLoaded && (
            <div className="w-2 h-2 bg-ai-primary rounded-full animate-pulse-soft ml-auto"></div>
          )}
        </div>
      </div>

      {/* Natural Language Input */}
      <div className="p-4 border-b border-terminal-border flex-shrink-0">
        <div className="space-y-3">
          <div className="flex items-center space-x-2">
            <MessageSquare className="w-4 h-4 text-ai-secondary" />
            <span className="text-sm font-medium text-terminal-text">Describe What You Want</span>
          </div>
          
          <div className="space-y-2">
            <textarea
              value={naturalLanguageInput}
              onChange={(e) => setNaturalLanguageInput(e.target.value)}
              placeholder="Try: 'show me all large files', 'find files modified today', 'what's using the most memory', 'install node dependencies', 'check git status', 'list running processes on port 3000'"
              className="w-full bg-terminal-bg border border-terminal-border rounded-md px-3 py-2 text-sm text-terminal-text resize-none focus-ring"
              rows={3}
              disabled={!isModelLoaded}
            />
            
            <button
              onClick={handleNaturalLanguageSubmit}
              disabled={!isModelLoaded || !naturalLanguageInput.trim() || isProcessing}
              className="w-full bg-ai-primary hover:bg-ai-primary/80 disabled:opacity-50 disabled:cursor-not-allowed text-white px-3 py-2 rounded-md text-sm font-medium transition-colors focus-ring"
            >
              {isProcessing ? 'Processing...' : 'Convert to Command'}
            </button>
          </div>
        </div>
      </div>

      {/* AI Suggestions */}
      <div className="flex-1 overflow-y-auto min-h-0">
        <div className="p-4">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center space-x-2">
              <Lightbulb className="w-4 h-4 text-ai-secondary" />
              <span className="text-sm font-medium text-terminal-text">Smart Suggestions</span>
            </div>
            
            {suggestions.length > 0 && (
              <button
                onClick={clearSuggestions}
                className="text-xs text-terminal-muted hover:text-terminal-text transition-colors"
              >
                Clear
              </button>
            )}
          </div>

          {!isModelLoaded ? (
            <div className="text-center py-8">
              <AlertCircle className="w-8 h-8 mx-auto text-terminal-muted mb-2" />
              <p className="text-sm text-terminal-muted">AI model loading...</p>
            </div>
          ) : suggestions.length === 0 ? (
            <div className="text-center py-8">
              <Zap className="w-8 h-8 mx-auto text-terminal-muted mb-2 opacity-50" />
              <p className="text-sm text-terminal-muted">
                Execute commands to get AI suggestions
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {suggestions.map((suggestion) => (
                <div
                  key={suggestion.id}
                  className="ai-suggestion transition-colors"
                >
                  <div className="flex items-start justify-between mb-2">
                    <span className="text-xs font-medium text-ai-primary capitalize">
                      {suggestion.type}
                    </span>
                    <div className="flex items-center space-x-2">
                      <span className="text-xs text-terminal-muted">
                        {Math.round(suggestion.confidence * 100)}%
                      </span>
                      {suggestion.type === 'command' && (
                        <button
                          onClick={() => {
                            navigator.clipboard.writeText(suggestion.content.includes('→') 
                              ? suggestion.content.split('→ ')[1] 
                              : suggestion.content);
                          }}
                          className="p-1 hover:bg-ai-primary/20 rounded transition-colors"
                          title="Copy command"
                        >
                          <Copy className="w-3 h-3 text-ai-primary" />
                        </button>
                      )}
                    </div>
                  </div>
                  
                  <p className="text-sm text-terminal-text leading-relaxed">
                    {suggestion.content}
                  </p>
                  
                  <div className="flex items-center justify-between mt-2">
                    <div className="text-xs text-terminal-muted">
                      {new Date(suggestion.timestamp).toLocaleTimeString()}
                    </div>
                    
                    {/* Feedback buttons for learning */}
                    <div className="flex items-center space-x-1">
                      <button
                        onClick={() => updateFeedback(suggestion.content, 1.0)}
                        className="p-1 hover:bg-green-500/20 rounded transition-colors"
                        title="This was helpful"
                      >
                        <ThumbsUp className="w-3 h-3 text-green-400" />
                      </button>
                      <button
                        onClick={() => updateFeedback(suggestion.content, 0.0)}
                        className="p-1 hover:bg-red-500/20 rounded transition-colors"
                        title="This was not helpful"
                      >
                        <ThumbsDown className="w-3 h-3 text-red-400" />
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="p-4 border-t border-terminal-border">
        <div className="space-y-2">
          <h3 className="text-xs font-medium text-terminal-muted uppercase tracking-wide">
            Quick Actions
          </h3>
          
          <div className="grid grid-cols-2 gap-2">
            <button 
              onClick={() => handleQuickAction('explain')}
              disabled={!isModelLoaded || commandHistory.length === 0 || isProcessing || quickActionLoading !== null}
              className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {quickActionLoading === 'explain' ? 'Explaining...' : 'Explain Last'}
            </button>
            
            <button 
              onClick={() => handleQuickAction('fix')}
              disabled={!isModelLoaded || commandHistory.length === 0 || isProcessing || quickActionLoading !== null}
              className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {quickActionLoading === 'fix' ? 'Fixing...' : 'Fix Error'}
            </button>
            
            <button 
              onClick={() => handleQuickAction('optimize')}
              disabled={!isModelLoaded || commandHistory.length === 0 || isProcessing || quickActionLoading !== null}
              className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {quickActionLoading === 'optimize' ? 'Optimizing...' : 'Optimize'}
            </button>
            
            <button 
              onClick={() => handleQuickAction('analyze')}
              disabled={!isModelLoaded || commandHistory.length === 0 || isProcessing || quickActionLoading !== null}
              className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {quickActionLoading === 'analyze' ? 'Analyzing...' : 'Analyze'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
