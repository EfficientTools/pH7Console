import { hasUnsafeTerminalCharacters } from './terminalInput';

export interface ExternalTerminalRequest {
  command?: string;
  workingDirectory?: string;
}

const MAX_DEEP_LINK_LENGTH = 64 * 1024;
const MAX_ENCODED_VALUE_LENGTH = 32 * 1024;
const MAX_COMMAND_BYTES = 8 * 1024;
const MAX_WORKING_DIRECTORY_BYTES = 16 * 1024;

function decodeBase64Url(value: string, maxDecodedBytes: number): string | null {
  if (
    value.length === 0 ||
    value.length > MAX_ENCODED_VALUE_LENGTH ||
    !/^[A-Za-z0-9_-]+$/.test(value)
  ) {
    return null;
  }

  try {
    const base64 = value.replace(/-/g, '+').replace(/_/g, '/');
    const padded = base64.padEnd(Math.ceil(base64.length / 4) * 4, '=');
    const binary = globalThis.atob(padded);
    if (binary.length === 0 || binary.length > maxDecodedBytes) return null;

    const bytes = Uint8Array.from(binary, character => character.charCodeAt(0));
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}

/**
 * Parse an external terminal request as inert text.
 *
 * Custom URL schemes are untrusted input. In particular, a carriage return,
 * newline, escape sequence, or other terminal control byte would turn a
 * supposedly review-only command into executable PTY input. Reject those
 * bytes before a session is created or anything is written to a terminal.
 */
export function parseExternalTerminalUrl(value: string): ExternalTerminalRequest | null {
  if (value.length === 0 || value.length > MAX_DEEP_LINK_LENGTH) return null;

  try {
    const url = new URL(value);
    if (
      url.protocol !== 'ph7console:' ||
      url.hostname !== 'new' ||
      url.username !== '' ||
      url.password !== '' ||
      url.port !== '' ||
      (url.pathname !== '' && url.pathname !== '/') ||
      url.hash !== ''
    ) {
      return null;
    }

    const allowedParameters = new Set(['cwd64', 'command64']);
    for (const key of url.searchParams.keys()) {
      if (!allowedParameters.has(key) || url.searchParams.getAll(key).length !== 1) return null;
    }

    const encodedWorkingDirectory = url.searchParams.get('cwd64');
    const workingDirectory = encodedWorkingDirectory === null
      ? undefined
      : decodeBase64Url(encodedWorkingDirectory, MAX_WORKING_DIRECTORY_BYTES) ?? undefined;
    if (
      encodedWorkingDirectory !== null &&
      (workingDirectory === undefined || hasUnsafeTerminalCharacters(workingDirectory))
    ) {
      return null;
    }

    const encodedCommand = url.searchParams.get('command64');
    const command = encodedCommand === null
      ? undefined
      : decodeBase64Url(encodedCommand, MAX_COMMAND_BYTES) ?? undefined;
    if (
      encodedCommand !== null &&
      (command === undefined || command.trim() === '' || hasUnsafeTerminalCharacters(command))
    ) {
      return null;
    }

    return { command, workingDirectory };
  } catch {
    return null;
  }
}
