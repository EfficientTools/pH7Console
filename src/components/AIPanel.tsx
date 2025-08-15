import React from 'react';
import { useAIStore } from '../store/aiStore';
import { useTerminalStore } from '../store/terminalStore';
import { Brain, Lightbulb, AlertCircle, Zap, MessageSquare } from 'lucide-react';

export const AIPanel: React.FC = () => {
  const { 
    isModelLoaded, 
    suggestions, 
    isProcessing, 
    clearSuggestions,
    translateNaturalLanguage 
  } = useAIStore();
  
  const { activeSession, commandHistory } = useTerminalStore();
  const [naturalLanguageInput, setNaturalLanguageInput] = React.useState('');

  const handleNaturalLanguageSubmit = async () => {
  if (!naturalLanguageInput.trim() || !activeSession) return;
  const context = commandHistory.slice(-3).map(cmd => cmd.command).join('; ');
  await translateNaturalLanguage(naturalLanguageInput, context);
  // Add as suggestion
  // Implementation would add to suggestions store
  setNaturalLanguageInput('');
  };

  return (
    <div className="h-full bg-terminal-surface flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-terminal-border">
        <div className="flex items-center space-x-2">
          <Brain className="w-5 h-5 text-ai-primary" />
          <h2 className="font-semibold text-terminal-text">AI Assistant</h2>
          {isModelLoaded && (
            <div className="w-2 h-2 bg-ai-primary rounded-full animate-pulse-soft ml-auto"></div>
          )}
        </div>
      </div>

      {/* Natural Language Input */}
      <div className="p-4 border-b border-terminal-border">
        <div className="space-y-3">
          <div className="flex items-center space-x-2">
            <MessageSquare className="w-4 h-4 text-ai-secondary" />
            <span className="text-sm font-medium text-terminal-text">Describe What You Want</span>
          </div>
          
          <div className="space-y-2">
            <textarea
              value={naturalLanguageInput}
              onChange={(e) => setNaturalLanguageInput(e.target.value)}
              placeholder="e.g., 'show me all large files in this directory'"
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
      <div className="flex-1 overflow-y-auto">
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
                  className="ai-suggestion cursor-pointer hover:bg-ai-primary/5 transition-colors"
                >
                  <div className="flex items-start justify-between mb-2">
                    <span className="text-xs font-medium text-ai-primary capitalize">
                      {suggestion.type}
                    </span>
                    <span className="text-xs text-terminal-muted">
                      {Math.round(suggestion.confidence * 100)}%
                    </span>
                  </div>
                  
                  <p className="text-sm text-terminal-text leading-relaxed">
                    {suggestion.content}
                  </p>
                  
                  <div className="mt-2 text-xs text-terminal-muted">
                    {new Date(suggestion.timestamp).toLocaleTimeString()}
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
            <button className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring">
              Explain Last
            </button>
            
            <button className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring">
              Fix Error
            </button>
            
            <button className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring">
              Optimize
            </button>
            
            <button className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring">
              Analyze
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
