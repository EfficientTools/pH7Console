import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { Terminal as XTermTerminal } from '@xterm/xterm';
import {
  ChevronDown,
  ChevronUp,
  Copy,
  FolderOpen,
  History,
  Search,
  RotateCcw,
  Trash2,
  X,
} from 'lucide-react';
import '@xterm/xterm/css/xterm.css';

import { useSettingsStore } from '../store/settingsStore';
import { CommandExecution, useTerminalStore } from '../store/terminalStore';
import { parseShellCommandEvent } from '../utils/shellIntegration';
import { hasUnsafeTerminalCharacters } from '../utils/terminalInput';
import { HistoryModal } from './HistoryModal';
import TerminalHeader from './TerminalHeader';

interface TerminalOutputEvent {
  sessionId: string;
  sequence: number;
  dataBase64: string;
}

interface TerminalOutputChunk {
  sequence: number;
  data: Uint8Array;
}

interface TerminalExitEvent {
  sessionId: string;
  exitCode: number;
  signal?: string;
}

interface TerminalSnapshot {
  dataBase64: string;
  lastSequence: number;
  isRunning: boolean;
  processId?: number;
}

type SessionStatus = 'connecting' | 'running' | 'exited' | 'error';

interface TerminalController {
  clear: () => void;
  copySelection: () => Promise<void>;
  findNext: (query: string) => boolean;
  findPrevious: (query: string) => boolean;
  fitAndFocus: () => void;
}

interface InputMessage {
  kind: 'text' | 'binary';
  text?: string;
  bytes?: number[];
  byteLength: number;
}

interface LiveTerminalSessionProps {
  active: boolean;
  fontFamily: string;
  fontSize: number;
  onController: (sessionId: string, controller: TerminalController | null) => void;
  onCommandCompleted: (execution: CommandExecution) => void;
  onStatusChange: (sessionId: string, status: SessionStatus) => void;
  onWorkingDirectory: (sessionId: string, workingDirectory: string) => void;
  sessionId: string;
}

const TERMINAL_THEME = {
  background: '#0a0d12',
  foreground: '#e8edf4',
  cursor: '#7c6df2',
  cursorAccent: '#0a0d12',
  selectionBackground: '#6d5df044',
  selectionInactiveBackground: '#6d5df022',
  black: '#151922',
  red: '#ff6b7a',
  green: '#64d98b',
  yellow: '#f4c95d',
  blue: '#6ca8ff',
  magenta: '#b88cff',
  cyan: '#5dd6d0',
  white: '#d8dee9',
  brightBlack: '#677080',
  brightRed: '#ff8995',
  brightGreen: '#83e5a2',
  brightYellow: '#ffdb7a',
  brightBlue: '#8ebcff',
  brightMagenta: '#caa5ff',
  brightCyan: '#7ce4de',
  brightWhite: '#ffffff',
};

const MAX_PENDING_INPUT_BYTES = 2 * 1024 * 1024;
const MAX_PENDING_OUTPUT_BYTES = 8 * 1024 * 1024;
const MAX_TEXT_INPUT_CHARS = 128 * 1024;

function decodeBase64(value: string): Uint8Array {
  if (!value) return new Uint8Array();
  const binary = window.atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function pathFromOsc7(value: string): string | null {
  try {
    const url = new URL(value);
    if (url.protocol !== 'file:') return null;
    return decodeURIComponent(url.pathname);
  } catch {
    return null;
  }
}

function decodeStreamFrame(payload: ArrayBuffer | Uint8Array): TerminalOutputChunk | null {
  const bytes = payload instanceof Uint8Array ? payload : new Uint8Array(payload);
  if (bytes.length < 9 || bytes[0] !== 1) return null;
  let sequence = 0;
  for (let index = 1; index <= 8; index += 1) {
    sequence = sequence * 256 + bytes[index];
  }
  return { sequence, data: bytes.slice(9) };
}

const LiveTerminalSession: React.FC<LiveTerminalSessionProps> = ({
  active,
  fontFamily,
  fontSize,
  onController,
  onCommandCompleted,
  onStatusChange,
  onWorkingDirectory,
  sessionId,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XTermTerminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const webglAddonRef = useRef<{ dispose: () => void } | null>(null);
  const activeRef = useRef(active);
  const [terminalGeneration, setTerminalGeneration] = useState(0);

  useEffect(() => {
    activeRef.current = active;
    const terminal = terminalRef.current;
    const fitAddon = fitAddonRef.current;
    if (!terminal || !fitAddon) return;
    terminal.options.cursorBlink = active;
    if (!active) return;

    const frame = window.requestAnimationFrame(() => {
      fitAddon.fit();
      terminal.focus();
    });
    return () => window.cancelAnimationFrame(frame);
  }, [active]);

  useEffect(() => {
    let cancelled = false;
    const terminal = terminalRef.current;

    if (!active || !terminal) {
      webglAddonRef.current?.dispose();
      webglAddonRef.current = null;
      return;
    }

    // Hidden tabs keep their PTY and canvas scrollback alive without holding
    // scarce GPU contexts. The active tab is promoted to WebGL on demand and
    // safely falls back to xterm's canvas renderer.
    void import('@xterm/addon-webgl')
      .then(({ WebglAddon }) => {
        if (cancelled || terminalRef.current !== terminal) return;
        try {
          const addon = new WebglAddon();
          addon.onContextLoss(() => {
            addon.dispose();
            if (webglAddonRef.current === addon) webglAddonRef.current = null;
          });
          terminal.loadAddon(addon);
          webglAddonRef.current = addon;
        } catch {
          // WebKit and virtualized test environments may not expose WebGL.
        }
      })
      .catch(() => {
        // Canvas rendering remains the supported fallback.
      });

    return () => {
      cancelled = true;
      webglAddonRef.current?.dispose();
      webglAddonRef.current = null;
    };
  }, [active, terminalGeneration]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    let disposed = false;
    let snapshotReady = false;
    let lastSequence = 0;
    let unlistenOutput: UnlistenFn | undefined;
    let unlistenExit: UnlistenFn | undefined;
    let streamSubscriberId: number | undefined;
    const queuedOutput: TerminalOutputChunk[] = [];
    let queuedOutputBytes = 0;
    let outputQueueOverflowed = false;
    const inputQueue: InputMessage[] = [];
    let pendingInputBytes = 0;
    let inputOverflowReported = false;
    let sendingInput = false;
    let flushScheduled = false;
    let commandStartedAt: number | null = null;
    let recordShellEvents = false;

    const terminal = new XTermTerminal({
      allowTransparency: false,
      cursorBlink: activeRef.current,
      cursorStyle: 'block',
      drawBoldTextInBrightColors: true,
      fastScrollSensitivity: 5,
      fontFamily,
      fontSize,
      letterSpacing: 0,
      lineHeight: 1.12,
      macOptionClickForcesSelection: true,
      minimumContrastRatio: 4.5,
      rightClickSelectsWord: true,
      screenReaderMode: true,
      scrollback: 20_000,
      scrollOnUserInput: true,
      tabStopWidth: 8,
      theme: TERMINAL_THEME,
    });
    const fitAddon = new FitAddon();
    const searchAddon = new SearchAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(searchAddon);
    terminal.loadAddon(new WebLinksAddon());
    terminal.open(container);
    terminalRef.current = terminal;
    fitAddonRef.current = fitAddon;
    setTerminalGeneration(generation => generation + 1);

    const flushInput = async () => {
      flushScheduled = false;
      if (sendingInput || disposed) return;
      sendingInput = true;
      try {
        while (inputQueue.length > 0 && !disposed) {
          const message = inputQueue.shift();
          if (!message) break;
          pendingInputBytes = Math.max(0, pendingInputBytes - message.byteLength);
          if (pendingInputBytes < MAX_PENDING_INPUT_BYTES / 2) inputOverflowReported = false;
          if (message.kind === 'text') {
            await invoke('write_to_terminal', {
              sessionId,
              data: message.text ?? '',
            });
          } else {
            await invoke('write_bytes_to_terminal', {
              sessionId,
              data: message.bytes ?? [],
            });
          }
        }
      } catch (error) {
        onStatusChange(sessionId, 'error');
        terminal.writeln(`\r\n\x1b[31m[pH7Console input error: ${String(error)}]\x1b[0m`);
      } finally {
        sendingInput = false;
        if (inputQueue.length > 0 && !disposed) scheduleInputFlush();
      }
    };

    const scheduleInputFlush = () => {
      if (flushScheduled || sendingInput || disposed) return;
      flushScheduled = true;
      window.queueMicrotask(flushInput);
    };

    const enqueueText = (data: string) => {
      if (data.length > MAX_TEXT_INPUT_CHARS) {
        let offset = 0;
        while (offset < data.length) {
          let end = Math.min(offset + MAX_TEXT_INPUT_CHARS, data.length);
          const finalCodeUnit = data.charCodeAt(end - 1);
          if (end < data.length && finalCodeUnit >= 0xd800 && finalCodeUnit <= 0xdbff) {
            end -= 1;
          }
          enqueueText(data.slice(offset, end));
          offset = end;
        }
        return;
      }
      const byteLength = new TextEncoder().encode(data).byteLength;
      if (pendingInputBytes + byteLength > MAX_PENDING_INPUT_BYTES) {
        if (!inputOverflowReported) {
          inputOverflowReported = true;
          terminal.writeln('\r\n\x1b[33m[pH7Console: input queue is full; paste less data at once]\x1b[0m');
        }
        return;
      }
      pendingInputBytes += byteLength;
      const last = inputQueue[inputQueue.length - 1];
      if (last?.kind === 'text' && (last.text?.length ?? 0) < 64 * 1024) {
        last.text = `${last.text ?? ''}${data}`;
        last.byteLength += byteLength;
      } else {
        inputQueue.push({ kind: 'text', text: data, byteLength });
      }
      scheduleInputFlush();
    };

    terminal.onData(enqueueText);
    terminal.onBinary(data => {
      if (pendingInputBytes + data.length > MAX_PENDING_INPUT_BYTES) {
        if (!inputOverflowReported) {
          inputOverflowReported = true;
          terminal.writeln('\r\n\x1b[33m[pH7Console: input queue is full]\x1b[0m');
        }
        return;
      }
      pendingInputBytes += data.length;
      inputQueue.push({
        kind: 'binary',
        bytes: Array.from(data, character => character.charCodeAt(0) & 0xff),
        byteLength: data.length,
      });
      scheduleInputFlush();
    });

    terminal.parser.registerOscHandler(7, data => {
      const workingDirectory = pathFromOsc7(data);
      if (workingDirectory) onWorkingDirectory(sessionId, workingDirectory);
      return true;
    });

    terminal.parser.registerOscHandler(133, data => {
      if (recordShellEvents && data === 'C') commandStartedAt = performance.now();
      return true;
    });

    terminal.parser.registerOscHandler(1337, data => {
      if (!recordShellEvents || disposed) return true;
      const event = parseShellCommandEvent(data);
      if (!event) return true;
      const durationMs = commandStartedAt === null
        ? 0
        : Math.max(0, Math.round(performance.now() - commandStartedAt));
      commandStartedAt = null;
      void invoke<CommandExecution>('record_shell_command', {
        sessionId,
        command: event.command,
        exitCode: event.exitCode,
        durationMs,
      }).then(execution => {
        if (!disposed) onCommandCompleted(execution);
      }).catch(error => {
        console.error('Could not record local command metadata:', error);
      });
      return true;
    });

    terminal.attachCustomKeyEventHandler(event => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
        window.dispatchEvent(new CustomEvent('ph7-terminal-search'));
        return false;
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        terminal.clear();
        return false;
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c' && terminal.hasSelection()) {
        void navigator.clipboard.writeText(terminal.getSelection());
        return false;
      }
      return true;
    });

    const controller: TerminalController = {
      clear: () => terminal.clear(),
      copySelection: async () => {
        if (terminal.hasSelection()) {
          await navigator.clipboard.writeText(terminal.getSelection());
        }
      },
      findNext: query => searchAddon.findNext(query, { incremental: true }),
      findPrevious: query => searchAddon.findPrevious(query),
      fitAndFocus: () => {
        fitAddon.fit();
        terminal.focus();
      },
    };
    onController(sessionId, controller);

    let resizeFrame: number | null = null;
    const resizeObserver = new ResizeObserver(() => {
      if (disposed || !activeRef.current || resizeFrame !== null) return;
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = null;
        if (!disposed && activeRef.current) fitAddon.fit();
      });
    });
    resizeObserver.observe(container);

    const resizeDisposable = terminal.onResize(({ cols, rows }) => {
      void invoke('resize_terminal', { sessionId, cols, rows });
    });

    const focusListener = () => {
      if (activeRef.current) controller.fitAndFocus();
    };
    window.addEventListener('ph7-focus-terminal', focusListener);

    const writeChunk = (output: TerminalOutputChunk) => {
      if (output.sequence <= lastSequence) return;
      terminal.write(output.data);
      lastSequence = output.sequence;
    };

    const queueOutput = (output: TerminalOutputChunk) => {
      queuedOutput.push(output);
      queuedOutputBytes += output.data.byteLength;
      while (queuedOutputBytes > MAX_PENDING_OUTPUT_BYTES && queuedOutput.length > 0) {
        const discarded = queuedOutput.shift();
        queuedOutputBytes = Math.max(0, queuedOutputBytes - (discarded?.data.byteLength ?? 0));
        outputQueueOverflowed = true;
      }
    };

    const initializeStream = async () => {
      try {
        const channel = new Channel<ArrayBuffer | Uint8Array>();
        channel.onmessage = payload => {
          if (disposed) return;
          const output = decodeStreamFrame(payload);
          if (!output) return;
          if (!snapshotReady) queueOutput(output);
          else writeChunk(output);
        };
        const subscriberId = await invoke<number>('attach_terminal_stream', {
          sessionId,
          onEvent: channel,
        });
        if (disposed) {
          void invoke('detach_terminal_stream', { sessionId, subscriberId });
          return;
        }
        streamSubscriberId = subscriberId;
      } catch {
        // Older native builds can still use the event transport. The channel
        // path is preferred because large binary chunks bypass JSON/base64.
        const stopOutput = await listen<TerminalOutputEvent>('terminal-output', event => {
          const output = event.payload;
          if (output.sessionId !== sessionId || disposed) return;
          const chunk = {
            sequence: output.sequence,
            data: decodeBase64(output.dataBase64),
          };
          if (!snapshotReady) queueOutput(chunk);
          else writeChunk(chunk);
        });
        if (disposed) {
          stopOutput();
          return;
        }
        unlistenOutput = stopOutput;
      }

      const stopExit = await listen<TerminalExitEvent>('terminal-exit', event => {
        const exit = event.payload;
        if (exit.sessionId !== sessionId || disposed) return;
        onStatusChange(sessionId, 'exited');
        const reason = exit.signal ? ` (${exit.signal})` : '';
        terminal.writeln(
          `\r\n\x1b[2m[pH7Console: shell exited with code ${exit.exitCode}${reason}]\x1b[0m`,
        );
      });
      if (disposed) {
        stopExit();
        return;
      }
      unlistenExit = stopExit;
      try {
        let snapshot = await invoke<TerminalSnapshot>('get_terminal_snapshot', { sessionId });
        // If a command flooded more data than the attach queue can safely
        // retain, take a fresh bounded native snapshot before rendering.
        for (let attempt = 0; outputQueueOverflowed && attempt < 2; attempt += 1) {
          queuedOutput.length = 0;
          queuedOutputBytes = 0;
          outputQueueOverflowed = false;
          snapshot = await invoke<TerminalSnapshot>('get_terminal_snapshot', { sessionId });
        }
        if (disposed) return;
        terminal.write(decodeBase64(snapshot.dataBase64));
        lastSequence = snapshot.lastSequence;
        snapshotReady = true;
        // Snapshot replay may contain old OSC metadata. Start recording only
        // after replay so reconnecting never duplicates command history.
        recordShellEvents = true;
        queuedOutput
          .sort((left, right) => left.sequence - right.sequence)
          .forEach(writeChunk);
        queuedOutput.length = 0;
        queuedOutputBytes = 0;
        onStatusChange(sessionId, snapshot.isRunning ? 'running' : 'exited');
        if (activeRef.current) {
          fitAddon.fit();
          terminal.focus();
        }
      } catch (error) {
        snapshotReady = true;
        onStatusChange(sessionId, 'error');
        terminal.writeln(`\x1b[31mUnable to attach to terminal: ${String(error)}\x1b[0m`);
      }
    };

    onStatusChange(sessionId, 'connecting');
    void initializeStream();

    return () => {
      disposed = true;
      unlistenOutput?.();
      unlistenExit?.();
      if (streamSubscriberId !== undefined) {
        void invoke('detach_terminal_stream', { sessionId, subscriberId: streamSubscriberId });
      }
      resizeObserver.disconnect();
      if (resizeFrame !== null) window.cancelAnimationFrame(resizeFrame);
      resizeDisposable.dispose();
      window.removeEventListener('ph7-focus-terminal', focusListener);
      onController(sessionId, null);
      webglAddonRef.current?.dispose();
      webglAddonRef.current = null;
      terminal.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [fontFamily, fontSize, onCommandCompleted, onController, onStatusChange, onWorkingDirectory, sessionId]);

  return (
    <div
      ref={containerRef}
      className={`absolute inset-0 px-3 py-2 ${active ? 'visible' : 'invisible pointer-events-none'}`}
      aria-hidden={!active}
      data-session-id={sessionId}
    />
  );
};

export const Terminal: React.FC = () => {
  const {
    activeSession,
    clearHistory,
    commandHistory,
    recordCommandExecution,
    restartSession,
    selectWorkspace,
    sessions,
    updateSessionWorkingDirectory,
  } = useTerminalStore();
  const { appearance } = useSettingsStore();
  const controllers = useRef(new Map<string, TerminalController>());
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [searchVisible, setSearchVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [historyVisible, setHistoryVisible] = useState(false);
  const [sessionStatuses, setSessionStatuses] = useState<Record<string, SessionStatus>>({});
  const [contextRefreshToken, setContextRefreshToken] = useState(0);
  const [restartingSession, setRestartingSession] = useState<string | null>(null);

  const activeSessionData = sessions.find(session => session.id === activeSession);
  const activeController = activeSession ? controllers.current.get(activeSession) : undefined;
  const activeStatus = activeSession ? sessionStatuses[activeSession] ?? 'connecting' : 'exited';

  const handleController = useCallback((sessionId: string, controller: TerminalController | null) => {
    if (controller) controllers.current.set(sessionId, controller);
    else controllers.current.delete(sessionId);
  }, []);

  const handleCommandCompleted = useCallback(
    (execution: CommandExecution) => {
      recordCommandExecution(execution);
      setContextRefreshToken(current => current + 1);
    },
    [recordCommandExecution],
  );

  const handleStatusChange = useCallback((sessionId: string, status: SessionStatus) => {
    setSessionStatuses(current => ({ ...current, [sessionId]: status }));
  }, []);

  const handleWorkingDirectory = useCallback(
    (sessionId: string, workingDirectory: string) => {
      void invoke<string>('sync_terminal_working_directory', {
        sessionId,
        workingDirectory,
      }).then(path => updateSessionWorkingDirectory(sessionId, path));
    },
    [updateSessionWorkingDirectory],
  );

  const handleRestart = useCallback(async (sessionId: string) => {
    if (restartingSession) return;
    setRestartingSession(sessionId);
    try {
      await restartSession(sessionId);
    } finally {
      setRestartingSession(null);
    }
  }, [restartSession, restartingSession]);

  const showSearch = useCallback(() => {
    setSearchVisible(true);
    window.requestAnimationFrame(() => searchInputRef.current?.focus());
  }, []);

  const hideSearch = useCallback(() => {
    setSearchVisible(false);
    setSearchQuery('');
    activeController?.fitAndFocus();
  }, [activeController]);

  const insertHistoryCommand = useCallback((command: string) => {
    if (!activeSession) return;
    if (!command.trim() || hasUnsafeTerminalCharacters(command)) {
      console.error('Refused to insert unsafe command-history text');
      return;
    }
    void invoke('write_to_terminal', {
      sessionId: activeSession,
      data: command,
    }).then(() => {
      window.dispatchEvent(new CustomEvent('ph7-focus-terminal'));
    }).catch(error => {
      console.error('Could not insert command from local history:', error);
    });
  }, [activeSession]);

  useEffect(() => {
    const listener = () => showSearch();
    window.addEventListener('ph7-terminal-search', listener);
    return () => window.removeEventListener('ph7-terminal-search', listener);
  }, [showSearch]);

  useEffect(() => {
    activeController?.fitAndFocus();
  }, [activeController, activeSession]);

  if (!activeSession || !activeSessionData) {
    return (
      <div className="h-full flex items-center justify-center text-terminal-muted" data-testid="terminal">
        <div className="text-center">
          <p className="text-sm">No active terminal session</p>
          <p className="mt-1 text-xs">Create a tab from the sidebar to start a shell.</p>
        </div>
      </div>
    );
  }

  const statusLabel = {
    connecting: 'Connecting',
    running: 'Live PTY',
    exited: 'Shell exited',
    error: 'Needs attention',
  }[activeStatus];

  return (
    <section
      className="h-full min-h-0 flex flex-col bg-terminal-bg"
      data-testid="terminal"
      aria-label="Terminal"
    >
      <TerminalHeader
        activeSessionId={activeSession}
        currentPath={activeSessionData.working_directory}
        onPathChange={path => updateSessionWorkingDirectory(activeSession, path)}
        refreshToken={contextRefreshToken}
      />

      <div className="h-10 shrink-0 flex items-center justify-between gap-2 border-b border-terminal-border px-3">
        <div className="min-w-0 flex items-center gap-2 text-xs text-terminal-muted">
          <span
            className={`h-2 w-2 rounded-full ${
              activeStatus === 'running'
                ? 'bg-emerald-400'
                : activeStatus === 'connecting'
                  ? 'bg-amber-400 animate-pulse'
                  : 'bg-red-400'
            }`}
            aria-hidden="true"
          />
          <span>{statusLabel}</span>
          <span className="hidden xl:inline text-terminal-muted/60">
            Persistent shell • ⌃C interrupt • ⌘F search • ⌘K clear
          </span>
        </div>

        <div className="flex items-center gap-1">
          {(activeStatus === 'exited' || activeStatus === 'error') && (
            <button
              type="button"
              onClick={() => void handleRestart(activeSession)}
              disabled={restartingSession === activeSession}
              className="terminal-toolbar-button text-emerald-300"
              aria-label="Restart shell"
              title="Restart this shell in the same workspace"
            >
              <RotateCcw className={`h-3.5 w-3.5 ${restartingSession === activeSession ? 'animate-spin' : ''}`} />
            </button>
          )}
          <button
            type="button"
            onClick={() => void selectWorkspace()}
            className="terminal-toolbar-button"
            aria-label="Choose workspace"
            title="Choose workspace"
          >
            <FolderOpen className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={showSearch}
            className="terminal-toolbar-button"
            aria-label="Search terminal output"
            title="Search output (⌘F)"
          >
            <Search className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={() => setHistoryVisible(true)}
            className="terminal-toolbar-button"
            aria-label="Open command history"
            title="Command history"
          >
            <History className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={() => void activeController?.copySelection()}
            className="terminal-toolbar-button"
            aria-label="Copy terminal selection"
            title="Copy selection"
          >
            <Copy className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={() => activeController?.clear()}
            className="terminal-toolbar-button"
            aria-label="Clear terminal scrollback"
            title="Clear scrollback (⌘K)"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {searchVisible && (
        <div className="h-11 shrink-0 flex items-center gap-1.5 border-b border-terminal-border bg-terminal-surface px-3">
          <Search className="h-3.5 w-3.5 text-terminal-muted" aria-hidden="true" />
          <input
            ref={searchInputRef}
            value={searchQuery}
            onChange={event => {
              const query = event.target.value;
              setSearchQuery(query);
              if (query) activeController?.findNext(query);
            }}
            onKeyDown={event => {
              if (event.key === 'Escape') hideSearch();
              if (event.key === 'Enter' && searchQuery) {
                if (event.shiftKey) activeController?.findPrevious(searchQuery);
                else activeController?.findNext(searchQuery);
              }
            }}
            className="min-w-0 flex-1 bg-transparent text-sm text-terminal-text outline-none"
            placeholder="Search visible output and scrollback"
            aria-label="Search terminal output"
          />
          <button
            type="button"
            onClick={() => searchQuery && activeController?.findPrevious(searchQuery)}
            className="terminal-toolbar-button"
            aria-label="Previous search result"
          >
            <ChevronUp className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={() => searchQuery && activeController?.findNext(searchQuery)}
            className="terminal-toolbar-button"
            aria-label="Next search result"
          >
            <ChevronDown className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={hideSearch}
            className="terminal-toolbar-button"
            aria-label="Close terminal search"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      <div className="relative min-h-0 flex-1 overflow-hidden ph7-xterm-host">
        {sessions.map(session => (
          <LiveTerminalSession
            key={session.id}
            active={session.id === activeSession}
            fontFamily={appearance.fontFamily}
            fontSize={appearance.fontSize}
            onCommandCompleted={handleCommandCompleted}
            onController={handleController}
            onStatusChange={handleStatusChange}
            onWorkingDirectory={handleWorkingDirectory}
            sessionId={session.id}
          />
        ))}
      </div>

      <HistoryModal
        isOpen={historyVisible}
        onClose={() => setHistoryVisible(false)}
        commandHistory={commandHistory}
        onClearHistory={clearHistory}
        onSelectCommand={insertHistoryCommand}
      />
    </section>
  );
};
