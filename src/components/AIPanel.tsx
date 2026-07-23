import React from 'react';
import { AIResponse, AISuggestion, useAIStore } from '../store/aiStore';
import { useTerminalStore } from '../store/terminalStore';
import { Brain, Lightbulb, AlertCircle, Zap, MessageSquare, ThumbsUp, ThumbsDown, Copy, Check, CornerDownLeft, Mic, Square, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { hasUnsafeTerminalCharacters } from '../utils/terminalInput';
import {
  initialVoiceInputState,
  VoiceInputEvent,
  voiceButtonLabel,
  voiceInputReducer,
} from '../voice/voiceInput';

interface CommandPlan {
  command: string;
  confidence: number;
  explanation: string;
  source: string;
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
  riskReasons: string[];
  requiresConfirmation: boolean;
  requiresStrongConfirmation: boolean;
}

type QuickAction = 'explain' | 'fix' | 'optimize' | 'analyze';

const sourceLabel = (source?: string) => {
  if (source === 'local_llm') return 'On-device model';
  if (source === 'deterministic') return 'Local fallback';
  if (source === 'literal') return 'Literal input';
  if (source === 'unavailable') return 'Unavailable';
  return 'Local';
};

export const AIPanel: React.FC = () => {
  const { 
    isModelLoaded, 
    realLlmStatus,
    modelError,
    suggestions, 
    isProcessing, 
    loadModel,
    refreshLlmStatus,
    clearSuggestions,
    addSuggestion,
    updateFeedback
  } = useAIStore();
  
  const { activeSession, commandHistory } = useTerminalStore();
  const [naturalLanguageInput, setNaturalLanguageInput] = React.useState('');
  const [quickActionLoading, setQuickActionLoading] = React.useState<string | null>(null);
  const [feedbackMessage, setFeedbackMessage] = React.useState<string | null>(null);
  const [copiedSuggestions, setCopiedSuggestions] = React.useState<Set<string>>(new Set());
  const [isPlanning, setIsPlanning] = React.useState(false);
  const [voiceState, dispatchVoiceEvent] = React.useReducer(voiceInputReducer, initialVoiceInputState);
  const aiBusyRef = React.useRef(false);
  const aiRequestRef = React.useRef(0);
  const naturalLanguageInputRef = React.useRef('');
  const voiceDraftPrefixRef = React.useRef('');
  const voicePhaseRef = React.useRef(voiceState.phase);
  const startAfterPermissionRef = React.useRef(false);

  React.useEffect(() => {
    naturalLanguageInputRef.current = naturalLanguageInput;
  }, [naturalLanguageInput]);

  React.useEffect(() => {
    voicePhaseRef.current = voiceState.phase;
  }, [voiceState.phase]);

  const beginVoiceCapture = React.useCallback(async () => {
    const existingDraft = naturalLanguageInputRef.current.trimEnd();
    voiceDraftPrefixRef.current = existingDraft ? `${existingDraft} ` : '';
    dispatchVoiceEvent({ kind: 'requesting', message: 'Starting the on-device microphone…' });
    try {
      await invoke('start_voice_input', { locale: navigator.language || undefined });
    } catch (error) {
      dispatchVoiceEvent({
        kind: 'error',
        message: typeof error === 'string' ? error : 'On-device voice input could not start.',
      });
    }
  }, []);

  const receiveVoiceEvent = React.useCallback((event: VoiceInputEvent) => {
    dispatchVoiceEvent(event);
    if ((event.kind === 'partial' || event.kind === 'final') && typeof event.transcript === 'string') {
      setNaturalLanguageInput(`${voiceDraftPrefixRef.current}${event.transcript}`);
      if (event.kind === 'final') voiceDraftPrefixRef.current = '';
    }
    if (event.kind === 'status' && startAfterPermissionRef.current) {
      startAfterPermissionRef.current = false;
      if (event.available) void beginVoiceCapture();
    }
  }, [beginVoiceCapture]);

  React.useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;

    const connectVoiceInput = async () => {
      unlisten = await listen<VoiceInputEvent>('voice-input', ({ payload }) => {
        if (!cancelled) receiveVoiceEvent(payload);
      });
      if (cancelled) {
        unlisten();
        return;
      }
      try {
        const status = await invoke<VoiceInputEvent>('get_voice_input_status', {
          locale: navigator.language || undefined,
        });
        receiveVoiceEvent(status);
      } catch (error) {
        receiveVoiceEvent({
          kind: 'error',
          message: typeof error === 'string' ? error : 'Could not check on-device voice input.',
        });
      }
    };

    void connectVoiceInput();
    return () => {
      cancelled = true;
      unlisten?.();
      if (voicePhaseRef.current === 'listening') void invoke('stop_voice_input');
    };
  }, [receiveVoiceEvent]);

  const handleVoiceInput = async () => {
    if (voiceState.phase === 'listening') {
      dispatchVoiceEvent({ kind: 'processing', message: 'Finishing the on-device transcript…' });
      try {
        await invoke('stop_voice_input');
      } catch (error) {
        dispatchVoiceEvent({
          kind: 'error',
          message: typeof error === 'string' ? error : 'Voice input could not be stopped cleanly.',
        });
      }
      return;
    }

    const hasAccess = voiceState.microphoneAuthorization === 'authorized'
      && voiceState.speechAuthorization === 'authorized';
    if (!hasAccess) {
      startAfterPermissionRef.current = true;
      dispatchVoiceEvent({ kind: 'requesting', message: 'Waiting for local microphone and speech permission…' });
      try {
        await invoke('request_voice_input_access');
      } catch (error) {
        startAfterPermissionRef.current = false;
        dispatchVoiceEvent({
          kind: 'error',
          message: typeof error === 'string' ? error : 'Voice permission could not be requested.',
        });
      }
      return;
    }

    await beginVoiceCapture();
  };

  const activeCommandHistory = React.useMemo(
    () => commandHistory.filter(command => command.session_id === activeSession),
    [activeSession, commandHistory]
  );
  const lastActiveCommand = activeCommandHistory[activeCommandHistory.length - 1];

  const addPlanSuggestions = React.useCallback((plan: CommandPlan) => {
    const timestamp = Date.now();
    addSuggestion({
      id: timestamp.toString(),
      type: 'command',
      content: plan.command,
      confidence: plan.confidence,
      timestamp,
      source: plan.source,
      riskLevel: plan.riskLevel,
      riskReasons: plan.riskReasons,
      requiresConfirmation: plan.requiresConfirmation,
      requiresStrongConfirmation: plan.requiresStrongConfirmation,
    });
    addSuggestion({
      id: `${timestamp}-explanation`,
      type: 'explanation',
      content: plan.explanation,
      confidence: plan.confidence,
      timestamp,
      source: plan.source,
    });
    if (plan.source !== 'local_llm') {
      void refreshLlmStatus().catch(() => undefined);
    }
  }, [addSuggestion, refreshLlmStatus]);

  const handleNaturalLanguageSubmit = async () => {
    if (!naturalLanguageInput.trim() || !activeSession || aiBusyRef.current) return;
    aiBusyRef.current = true;
    const requestId = ++aiRequestRef.current;
    setIsPlanning(true);
    try {
      const plan = await invoke<CommandPlan>('create_command_plan', {
        sessionId: activeSession,
        input: naturalLanguageInput,
      });
      if (requestId !== aiRequestRef.current) return;
      addPlanSuggestions(plan);
      setNaturalLanguageInput('');
    } catch (error) {
      if (requestId !== aiRequestRef.current) return;
      console.error('Natural language translation failed:', error);
      addSuggestion({
        id: Date.now().toString(),
        type: 'error',
        content: typeof error === 'string' ? error : 'Could not create a local command plan.',
        confidence: 0,
        source: 'unavailable',
        timestamp: Date.now(),
      });
    } finally {
      if (requestId === aiRequestRef.current) {
        aiBusyRef.current = false;
        setIsPlanning(false);
      }
    }
  };

  const handleCancelLocalAI = async () => {
    ++aiRequestRef.current;
    aiBusyRef.current = false;
    setIsPlanning(false);
    setQuickActionLoading(null);
    try {
      await invoke<boolean>('cancel_ai_generation');
    } catch (error) {
      console.error('Could not stop local AI generation:', error);
    }
  };

  const handleInsert = async (suggestion: AISuggestion) => {
    if (!activeSession) return;
    if (
      typeof suggestion.content !== 'string' ||
      !suggestion.content.trim() ||
      hasUnsafeTerminalCharacters(suggestion.content)
    ) {
      console.error('Refused to insert unsafe command-suggestion text');
      return;
    }
    await invoke('write_to_terminal', {
      sessionId: activeSession,
      data: suggestion.content,
    });
    window.dispatchEvent(new CustomEvent('ph7-focus-terminal'));
  };

  const handleCopy = async (suggestion: AISuggestion) => {
    try {
      // Extract the actual command from the suggestion content
      let textToCopy = suggestion.content;
      
      if (suggestion.content.includes('→')) {
        // Natural language → command format
        textToCopy = suggestion.content.split('→ ')[1] || suggestion.content;
      } else if (suggestion.content.startsWith('💡')) {
        // Remove emoji prefix
        textToCopy = suggestion.content.replace('💡 Natural Language → Command: ', '');
      }
      
      // Clean up any remaining formatting
      textToCopy = textToCopy.trim();
      
      // Copy to clipboard
      await navigator.clipboard.writeText(textToCopy);
      
      // Add suggestion ID to copied set
      setCopiedSuggestions(prev => new Set(prev).add(suggestion.id));
      
      // Remove the checkmark after 2 seconds
      setTimeout(() => {
        setCopiedSuggestions(prev => {
          const newSet = new Set(prev);
          newSet.delete(suggestion.id);
          return newSet;
        });
      }, 2000);
      
    } catch (error) {
      console.error('Failed to copy to clipboard:', error);
    }
  };

  const handleFeedback = async (suggestion: AISuggestion, isPositive: boolean) => {
    try {
      await updateFeedback(suggestion.id, suggestion.content, isPositive ? 1.0 : 0.0);
      
      setFeedbackMessage(isPositive ? 'Preference noted for this session' : 'Feedback noted for this session');
      setTimeout(() => setFeedbackMessage(null), 2000);
    } catch (error) {
      console.error('Feedback error:', error);
      setFeedbackMessage('Feedback failed to save');
      setTimeout(() => setFeedbackMessage(null), 2000);
    }
  };

  const handleQuickAction = async (action: QuickAction) => {
    const lastCommand = lastActiveCommand;
    if (!lastCommand || aiBusyRef.current) return;

    aiBusyRef.current = true;
    const requestId = ++aiRequestRef.current;
    setQuickActionLoading(action);

    try {
      switch (action) {
        case 'explain': {
          const response = await invoke<AIResponse>('ai_explain_command', {
            command: lastCommand.command
          });
          if (requestId !== aiRequestRef.current) return;
          addSuggestion({
            id: Date.now().toString(),
            type: 'explanation',
            content: response.text,
            confidence: response.confidence ?? 0,
            source: response.source,
            timestamp: Date.now(),
          });
          if (response.source !== 'local_llm') {
            void refreshLlmStatus().catch(() => undefined);
          }
          break;
        }
        case 'fix': {
          const errorEvidence = lastCommand.output.trim()
            ? ` Error output: ${lastCommand.output.slice(0, 4_000)}`
            : ` No output was captured; the exit status was ${lastCommand.exit_code ?? 'unknown'}.`;
          const plan = await invoke<CommandPlan>('create_command_plan', {
            sessionId: activeSession,
            input: `Create one conservative next command to diagnose or fix this failed command: ${lastCommand.command}.${errorEvidence}`,
          });
          if (requestId !== aiRequestRef.current) return;
          addPlanSuggestions(plan);
          break;
        }
        case 'optimize': {
          const plan = await invoke<CommandPlan>('create_command_plan', {
            sessionId: activeSession,
            input: `Create one command that optimizes this shell command while preserving its behavior and literal arguments: ${lastCommand.command}`,
          });
          if (requestId !== aiRequestRef.current) return;
          addPlanSuggestions(plan);
          break;
        }
        case 'analyze': {
          const response = await invoke<AIResponse>('ai_analyze_output', {
            command: lastCommand.command,
            output: lastCommand.output || ''
          });
          if (requestId !== aiRequestRef.current) return;
          addSuggestion({
            id: Date.now().toString(),
            type: 'analysis',
            content: response.text,
            confidence: response.confidence ?? 0,
            source: response.source,
            timestamp: Date.now(),
          });
          if (response.source !== 'local_llm') {
            void refreshLlmStatus().catch(() => undefined);
          }
          break;
        }
      }
    } catch (error) {
      if (requestId !== aiRequestRef.current) return;
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
        errorMessage += ' Local command intelligence is not ready yet.';
      }
      
      addSuggestion({
        id: Date.now().toString(),
        type: 'error',
        content: errorMessage,
        confidence: 0,
        source: 'unavailable',
        timestamp: Date.now(),
      });
    } finally {
      if (requestId === aiRequestRef.current) {
        aiBusyRef.current = false;
        setQuickActionLoading(null);
      }
    }
  };

  return (
    <div className="h-full bg-terminal-surface flex flex-col min-h-0">
      {/* Header */}
      <div className="p-4 border-b border-terminal-border flex-shrink-0">
        <div className="flex items-center space-x-2">
          <Brain className="w-5 h-5 text-ai-primary" />
          <h2 className="font-semibold text-terminal-text">Local Command Intelligence</h2>
          {isModelLoaded && (
            <span
              className={`ml-auto h-2 w-2 rounded-full ${
                realLlmStatus.available
                  ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.55)]'
                  : 'bg-amber-400 shadow-[0_0_8px_rgba(251,191,36,0.4)]'
              }`}
              aria-label={realLlmStatus.available ? 'On-device language model ready' : 'Deterministic local intelligence ready'}
              title={realLlmStatus.message}
              role="status"
            />
          )}
        </div>
        
        {/* Feedback Message */}
        {feedbackMessage && (
          <div className="mt-2 p-2 bg-ai-primary/20 border border-ai-primary/30 rounded-md text-xs text-ai-primary animate-fade-in" role="status" aria-live="polite">
            {feedbackMessage}
          </div>
        )}
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
              aria-label="Describe a command to plan"
            />

            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0 text-[11px] leading-4 text-terminal-muted" aria-live="polite" role="status">
                <span className="block text-emerald-300/90">On-device only · audio is never stored</span>
                <span className="block truncate" title={voiceState.message}>{voiceState.message}</span>
              </div>
              <button
                type="button"
                onClick={() => void handleVoiceInput()}
                disabled={
                  !isModelLoaded
                  || voiceState.phase === 'checking'
                  || voiceState.phase === 'requesting'
                  || voiceState.phase === 'processing'
                  || voiceState.phase === 'denied'
                  || voiceState.phase === 'unavailable'
                }
                aria-label={voiceButtonLabel(voiceState)}
                aria-pressed={voiceState.phase === 'listening'}
                title={voiceState.message}
                className={`flex flex-shrink-0 items-center gap-1.5 rounded-md border px-2.5 py-1.5 text-xs font-medium transition-colors focus-ring disabled:cursor-not-allowed disabled:opacity-50 ${
                  voiceState.phase === 'listening'
                    ? 'border-red-400/60 bg-red-500/20 text-red-200 hover:bg-red-500/30'
                    : 'border-ai-primary/40 bg-ai-primary/10 text-ai-primary hover:bg-ai-primary/20'
                }`}
              >
                {voiceState.phase === 'listening' ? (
                  <Square className="h-3.5 w-3.5 fill-current" />
                ) : voiceState.phase === 'checking' || voiceState.phase === 'requesting' || voiceState.phase === 'processing' ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Mic className="h-3.5 w-3.5" />
                )}
                {voiceState.phase === 'listening' ? 'Stop' : 'Voice'}
              </button>
            </div>
            
            <div className="flex gap-2">
              <button
                type="button"
                onClick={handleNaturalLanguageSubmit}
                disabled={!isModelLoaded || !naturalLanguageInput.trim() || isProcessing || isPlanning || quickActionLoading !== null}
                className="min-w-0 flex-1 bg-ai-primary hover:bg-ai-primary/80 disabled:opacity-50 disabled:cursor-not-allowed text-white px-3 py-2 rounded-md text-sm font-medium transition-colors focus-ring"
              >
                {isPlanning ? 'Planning...' : 'Create Safe Command Plan'}
              </button>
              {(isPlanning || quickActionLoading !== null) && (
                <button
                  type="button"
                  onClick={() => void handleCancelLocalAI()}
                  className="flex items-center gap-1.5 rounded-md border border-red-400/40 bg-red-500/10 px-3 py-2 text-sm font-medium text-red-200 transition-colors hover:bg-red-500/20 focus-ring"
                  aria-label="Stop local AI generation"
                  title="Stop on-device generation"
                >
                  <Square className="h-3.5 w-3.5 fill-current" />
                  Stop
                </button>
              )}
            </div>
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
                type="button"
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
              <p className="text-sm text-terminal-muted">
                {isProcessing ? 'AI model loading...' : 'Local AI is unavailable'}
              </p>
              {modelError && !isProcessing && (
                <button
                  type="button"
                  onClick={() => void loadModel()}
                  className="mt-3 px-3 py-1.5 text-xs bg-terminal-border text-terminal-text rounded hover:bg-terminal-muted/30 transition-colors"
                >
                  Retry AI
                </button>
              )}
            </div>
          ) : suggestions.length === 0 ? (
            <div className="text-center py-8">
              <Zap className="w-8 h-8 mx-auto text-terminal-muted mb-2 opacity-50" />
              <p className="text-sm text-terminal-muted">
                Describe a task above or use a quick action on this tab's last command.
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
                      <span className="rounded bg-terminal-border/60 px-1.5 py-0.5 text-[10px] text-terminal-muted">
                        {sourceLabel(suggestion.source)}
                      </span>
                      {suggestion.type === 'command' && (
                        <>
                          <span className={`rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide ${
                            suggestion.riskLevel === 'critical' || suggestion.riskLevel === 'high'
                              ? 'bg-red-500/20 text-red-300'
                              : suggestion.riskLevel === 'medium'
                                ? 'bg-amber-500/20 text-amber-300'
                                : suggestion.riskLevel === 'low'
                                  ? 'bg-emerald-500/20 text-emerald-300'
                                  : 'bg-terminal-border text-terminal-muted'
                          }`}>
                            {suggestion.riskLevel ?? 'unrated'}
                          </span>
                          <button
                            type="button"
                            onClick={() => void handleInsert(suggestion)}
                            className="p-1 rounded hover:bg-ai-primary/20 hover:scale-105 transition-all"
                            title="Insert into terminal without executing"
                            aria-label="Insert command without executing"
                          >
                            <CornerDownLeft className="w-3 h-3 text-ai-primary" />
                          </button>
                          <button
                            type="button"
                            onClick={() => handleCopy(suggestion)}
                            className={`p-1 rounded transition-all duration-200 relative ${
                              copiedSuggestions.has(suggestion.id)
                                ? 'bg-ai-primary/30 shadow-sm border border-ai-primary/40 animate-pulse-once'
                                : 'hover:bg-ai-primary/20 hover:scale-105'
                            }`}
                            title={copiedSuggestions.has(suggestion.id) ? 'Copied to clipboard!' : 'Copy command to clipboard'}
                            aria-label={copiedSuggestions.has(suggestion.id) ? 'Command copied' : 'Copy command to clipboard'}
                          >
                            {copiedSuggestions.has(suggestion.id) ? (
                              <Check className="w-3 h-3 text-green-400 animate-fade-in" />
                            ) : (
                              <Copy className="w-3 h-3 text-ai-primary transition-transform" />
                            )}
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                  
                  <p className={`text-sm text-terminal-text leading-relaxed ${suggestion.type === 'command' ? 'font-mono break-words' : ''}`}>
                    {suggestion.content}
                  </p>
                  {suggestion.type === 'command' && suggestion.riskReasons && suggestion.riskReasons.length > 0 && (
                    <p className="mt-2 text-xs text-terminal-muted">
                      {suggestion.riskReasons.join(' • ')}
                    </p>
                  )}
                  {suggestion.type === 'command' && suggestion.requiresStrongConfirmation && (
                    <p className="mt-2 rounded border border-red-400/30 bg-red-500/10 px-2 py-1.5 text-xs text-red-200">
                      High-impact plan. Inserting does not execute it; inspect every argument before pressing Enter.
                    </p>
                  )}
                  
                  <div className="flex items-center justify-between mt-2">
                    <div className="text-xs text-terminal-muted">
                      {new Date(suggestion.timestamp).toLocaleTimeString()}
                    </div>
                    
                    {/* Feedback adjusts memory-only preferences for this session. */}
                    <div className="flex items-center space-x-1">
                      <button
                        type="button"
                        onClick={() => handleFeedback(suggestion, true)}
                        className={`p-1 rounded transition-all duration-200 ${
                          suggestion.feedback === 'positive' 
                            ? 'bg-green-500/40 shadow-sm border border-green-400/30' 
                            : 'hover:bg-green-500/20'
                        }`}
                        title={suggestion.feedback === 'positive' ? 'You marked this as helpful' : 'Mark as helpful'}
                        aria-label="Mark suggestion as helpful"
                        aria-pressed={suggestion.feedback === 'positive'}
                      >
                        <ThumbsUp 
                          className={`w-3 h-3 transition-all duration-200 ${
                            suggestion.feedback === 'positive' 
                              ? 'text-green-300 drop-shadow-sm' 
                              : 'text-green-400'
                          }`} 
                          fill={suggestion.feedback === 'positive' ? 'currentColor' : 'none'}
                        />
                      </button>
                      <button
                        type="button"
                        onClick={() => handleFeedback(suggestion, false)}
                        className={`p-1 rounded transition-all duration-200 ${
                          suggestion.feedback === 'negative' 
                            ? 'bg-red-500/40 shadow-sm border border-red-400/30' 
                            : 'hover:bg-red-500/20'
                        }`}
                        title={suggestion.feedback === 'negative' ? 'You marked this as not helpful' : 'Mark as not helpful'}
                        aria-label="Mark suggestion as not helpful"
                        aria-pressed={suggestion.feedback === 'negative'}
                      >
                        <ThumbsDown 
                          className={`w-3 h-3 transition-all duration-200 ${
                            suggestion.feedback === 'negative' 
                              ? 'text-red-300 drop-shadow-sm' 
                              : 'text-red-400'
                          }`} 
                          fill={suggestion.feedback === 'negative' ? 'currentColor' : 'none'}
                        />
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
              type="button"
              onClick={() => handleQuickAction('explain')}
              disabled={!isModelLoaded || !lastActiveCommand || isProcessing || isPlanning || quickActionLoading !== null}
              className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {quickActionLoading === 'explain' ? 'Explaining...' : 'Explain Last'}
            </button>
            
            <button 
              type="button"
              onClick={() => handleQuickAction('fix')}
              disabled={!isModelLoaded || !lastActiveCommand || lastActiveCommand.exit_code == null || lastActiveCommand.exit_code === 0 || isProcessing || isPlanning || quickActionLoading !== null}
              title={lastActiveCommand?.exit_code ? 'Create a conservative plan for the last failed command' : 'The last command did not fail'}
              className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {quickActionLoading === 'fix' ? 'Fixing...' : 'Fix Error'}
            </button>
            
            <button 
              type="button"
              onClick={() => handleQuickAction('optimize')}
              disabled={!isModelLoaded || !lastActiveCommand || isProcessing || isPlanning || quickActionLoading !== null}
              className="px-3 py-2 bg-terminal-bg hover:bg-terminal-border text-xs text-terminal-text rounded border border-terminal-border transition-colors focus-ring disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {quickActionLoading === 'optimize' ? 'Optimizing...' : 'Optimize'}
            </button>
            
            <button 
              type="button"
              onClick={() => handleQuickAction('analyze')}
              disabled={!isModelLoaded || !lastActiveCommand?.output.trim() || isProcessing || isPlanning || quickActionLoading !== null}
              title={lastActiveCommand?.output.trim() ? 'Analyze captured output' : 'No command output was captured for this history entry'}
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
