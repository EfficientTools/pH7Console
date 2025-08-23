// Enhanced Smart Suggestions Component for pH7Console
// This component provides an intelligent, contextual suggestion panel

import React, { useState, useEffect } from 'react';
import { 
  Zap, 
  Lightbulb, 
  AlertTriangle, 
  TrendingUp, 
  Clock, 
  Target, 
  FileSearch,
  Monitor,
  GitBranch,
  Settings,
  ChevronRight,
  Copy,
  Play
} from 'lucide-react';

interface IntentPrediction {
  intent: string;
  confidence: number;
  context: string[];
  suggestions: string[];
  explanation: string;
}

interface SmartSuggestionsProps {
  input: string;
  sessionId: string;
  activeSession: string | null;
  onSelectSuggestion: (suggestion: string) => void;
  onExecuteSuggestion: (suggestion: string) => void;
  isVisible: boolean;
}

export const SmartSuggestions: React.FC<SmartSuggestionsProps> = ({
  input,
  activeSession,
  onSelectSuggestion,
  onExecuteSuggestion,
  isVisible
}) => {
  const [predictions, setPredictions] = useState<IntentPrediction[]>([]);
  const [proactiveSuggestions, setProactiveSuggestions] = useState<any[]>([]);
  const [systemContext, setSystemContext] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [selectedIndex, _setSelectedIndex] = useState(0);

  // Load predictions as user types
  useEffect(() => {
    if (!input.trim() || !activeSession) {
      setPredictions([]);
      return;
    }

    setLoading(true);
    const debounceTimer = setTimeout(async () => {
      try {
        // Simulate intelligent prediction API call
        // In real implementation, this would call intelligentPredictor.getPredictiveCompletions
        const mockPredictions: IntentPrediction[] = [
          {
            intent: 'file_search',
            confidence: 0.9,
            context: ['/Users/project', 'node'],
            suggestions: [
              'find . -name "*.js" -type f',
              'find . -type f -size +1M | head -10',
              'ls -la | grep -E "\\.(js|ts)$"'
            ],
            explanation: 'Search for files based on your input pattern'
          },
          {
            intent: 'development',
            confidence: 0.85,
            context: ['git_repo', 'node_project'],
            suggestions: [
              'npm run dev',
              'git status',
              'npm test',
              'npm run build'
            ],
            explanation: 'Development workflow commands for your Node.js project'
          }
        ];

        setPredictions(mockPredictions);
      } catch (error) {
        console.error('Failed to get predictions:', error);
      } finally {
        setLoading(false);
      }
    }, 300);

    return () => clearTimeout(debounceTimer);
  }, [input, activeSession]);

  // Load system context and proactive suggestions
  useEffect(() => {
    if (!activeSession) return;

    const loadContextAndSuggestions = async () => {
      try {
        // Load system context
        // const context = await invoke('get_enhanced_system_context', { sessionId });
        // setSystemContext(context);

        // Load proactive suggestions
        // const suggestions = await invoke('get_proactive_suggestions', { sessionId });
        // setProactiveSuggestions(suggestions);

        // Mock data for demonstration
        setSystemContext({
          disk_usage: 85,
          cpu_usage: 25,
          git_status: { has_changes: true, branch: 'main' },
          project_type: 'node'
        });

        setProactiveSuggestions([
          {
            suggestion_type: 'maintenance',
            priority: 0.8,
            description: 'Disk space is getting low',
            commands: ['du -sh * | sort -hr | head -10', 'find . -name "*.log" -size +10M'],
            trigger_condition: 'disk_usage > 80%'
          }
        ]);
      } catch (error) {
        console.error('Failed to load context:', error);
      }
    };

    loadContextAndSuggestions();
  }, [activeSession]);

  const getIntentIcon = (intent: string) => {
    switch (intent) {
      case 'file_search': return <FileSearch className="w-4 h-4" />;
      case 'development': return <GitBranch className="w-4 h-4" />;
      case 'system_monitor': return <Monitor className="w-4 h-4" />;
      case 'maintenance': return <Settings className="w-4 h-4" />;
      case 'performance': return <TrendingUp className="w-4 h-4" />;
      default: return <Lightbulb className="w-4 h-4" />;
    }
  };

  const getIntentColor = (intent: string) => {
    switch (intent) {
      case 'file_search': return 'text-blue-400';
      case 'development': return 'text-green-400';
      case 'system_monitor': return 'text-yellow-400';
      case 'maintenance': return 'text-orange-400';
      case 'performance': return 'text-red-400';
      default: return 'text-ai-primary';
    }
  };

  const handleCopySuggestion = async (suggestion: string) => {
    try {
      await navigator.clipboard.writeText(suggestion);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  if (!isVisible) return null;

  return (
    <div className="bg-terminal-bg border border-terminal-border rounded-lg shadow-lg overflow-hidden">
      {/* Header */}
      <div className="bg-gradient-to-r from-ai-primary/10 to-ai-secondary/10 p-3 border-b border-terminal-border">
        <div className="flex items-center space-x-2">
          <Zap className="w-5 h-5 text-ai-primary" />
          <span className="text-sm font-medium text-terminal-text">Intelligent Predictions</span>
          {loading && (
            <div className="w-3 h-3 bg-ai-primary rounded-full animate-pulse"></div>
          )}
        </div>
      </div>

      {/* System Alerts */}
      {proactiveSuggestions.length > 0 && (
        <div className="p-3 bg-yellow-500/10 border-b border-terminal-border">
          <div className="flex items-start space-x-2">
            <AlertTriangle className="w-4 h-4 text-yellow-400 mt-0.5" />
            <div className="flex-1 min-w-0">
              <p className="text-xs text-yellow-400 font-medium">System Alert</p>
              <p className="text-xs text-terminal-muted mt-1">
                {proactiveSuggestions[0].description}
              </p>
              <div className="flex space-x-1 mt-2">
                {proactiveSuggestions[0].commands.slice(0, 2).map((cmd: string, idx: number) => (
                  <button
                    key={idx}
                    onClick={() => onSelectSuggestion(cmd)}
                    className="px-2 py-1 bg-yellow-400/20 text-yellow-300 rounded text-xs hover:bg-yellow-400/30 transition-colors"
                  >
                    {cmd.split(' ')[0]}...
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Intent-based Predictions */}
      <div className="max-h-80 overflow-y-auto">
        {predictions.map((prediction, index) => (
          <div key={prediction.intent} className="p-3 hover:bg-terminal-border/50 transition-colors">
            {/* Intent Header */}
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center space-x-2">
                <div className={getIntentColor(prediction.intent)}>
                  {getIntentIcon(prediction.intent)}
                </div>
                <span className="text-sm font-medium text-terminal-text capitalize">
                  {prediction.intent.replace('_', ' ')}
                </span>
                <span className="text-xs text-terminal-muted">
                  {Math.round(prediction.confidence * 100)}% confident
                </span>
              </div>
            </div>

            {/* Context Tags */}
            <div className="flex space-x-1 mb-2">
              {prediction.context.slice(0, 3).map((ctx, idx) => (
                <span
                  key={idx}
                  className="px-2 py-1 bg-ai-primary/10 text-ai-primary rounded text-xs"
                >
                  {ctx}
                </span>
              ))}
            </div>

            {/* Explanation */}
            <p className="text-xs text-terminal-muted mb-3">{prediction.explanation}</p>

            {/* Suggestions */}
            <div className="space-y-1">
              {prediction.suggestions.slice(0, 3).map((suggestion, sugIndex) => (
                <div
                  key={sugIndex}
                  className={`flex items-center justify-between p-2 rounded transition-colors cursor-pointer ${
                    index === selectedIndex && sugIndex === 0
                      ? 'bg-ai-primary/20 border border-ai-primary/30'
                      : 'bg-terminal-bg hover:bg-terminal-border/30'
                  }`}
                  onClick={() => onSelectSuggestion(suggestion)}
                >
                  <div className="flex-1 min-w-0">
                    <code className="text-xs text-terminal-text font-mono">
                      {suggestion}
                    </code>
                  </div>
                  <div className="flex items-center space-x-1 ml-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleCopySuggestion(suggestion);
                      }}
                      className="p-1 text-terminal-muted hover:text-terminal-text transition-colors"
                      title="Copy to clipboard"
                    >
                      <Copy className="w-3 h-3" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onExecuteSuggestion(suggestion);
                      }}
                      className="p-1 text-terminal-muted hover:text-ai-primary transition-colors"
                      title="Execute command"
                    >
                      <Play className="w-3 h-3" />
                    </button>
                    <ChevronRight className="w-3 h-3 text-terminal-muted" />
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}

        {/* No Predictions */}
        {predictions.length === 0 && !loading && input.length > 0 && (
          <div className="p-6 text-center">
            <Clock className="w-8 h-8 mx-auto text-terminal-muted mb-2 opacity-50" />
            <p className="text-sm text-terminal-muted">
              Learning your patterns...
            </p>
            <p className="text-xs text-terminal-muted mt-1">
              Type more to get intelligent suggestions
            </p>
          </div>
        )}

        {/* Getting Started */}
        {predictions.length === 0 && !loading && input.length === 0 && (
          <div className="p-4">
            <div className="text-center mb-4">
              <Target className="w-8 h-8 mx-auto text-terminal-muted mb-2 opacity-50" />
              <p className="text-sm text-terminal-muted">
                Start typing to see intelligent suggestions
              </p>
            </div>
            
            <div className="space-y-2">
              <p className="text-xs text-terminal-muted font-medium">Try asking:</p>
              <div className="space-y-1">
                {[
                  '"show me large files"',
                  '"what is using my CPU?"',
                  '"find all JavaScript files"',
                  '"check git status"'
                ].map((example, idx) => (
                  <button
                    key={idx}
                    onClick={() => onSelectSuggestion(example.slice(1, -1))}
                    className="block w-full text-left px-2 py-1 text-xs text-ai-primary hover:bg-ai-primary/10 rounded transition-colors"
                  >
                    {example}
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="p-2 bg-terminal-bg border-t border-terminal-border">
        <div className="flex items-center justify-between text-xs text-terminal-muted">
          <span>AI-powered suggestions</span>
          <div className="flex items-center space-x-2">
            {systemContext && (
              <>
                <span>CPU: {systemContext.cpu_usage}%</span>
                <span>•</span>
                <span>Disk: {systemContext.disk_usage}%</span>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default SmartSuggestions;
