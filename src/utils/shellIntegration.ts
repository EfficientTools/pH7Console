import { hasUnsafeTerminalCharacters } from './terminalInput';

export interface ShellCommandEvent {
  command: string;
  exitCode: number;
  truncated: boolean;
}

const MAX_OSC_PAYLOAD_LENGTH = 48 * 1024;
const MAX_COMMAND_LENGTH = 8 * 1024;

/**
 * Parse pH7Console's private OSC 1337 command-completion payload as inert
 * metadata. It deliberately accepts only the documented, bounded protocol and
 * never turns terminal output into executable input.
 */
export function parseShellCommandEvent(payload: string): ShellCommandEvent | null {
  if (payload.length === 0 || payload.length > MAX_OSC_PAYLOAD_LENGTH) return null;
  const fields = payload.split(';');
  if (fields.shift() !== 'pH7') return null;

  const values = new Map<string, string>();
  for (const field of fields) {
    const separator = field.indexOf('=');
    if (separator <= 0) return null;
    const key = field.slice(0, separator);
    if (values.has(key)) return null;
    values.set(key, field.slice(separator + 1));
  }

  if (values.get('event') !== 'command_end') return null;
  const status = values.get('status');
  const truncated = values.get('truncated');
  const encodedCommand = values.get('command');
  if (!status || !/^[0-9]{1,3}$/.test(status)) return null;
  const exitCode = Number(status);
  if (!Number.isInteger(exitCode) || exitCode < 0 || exitCode > 255) return null;
  if (truncated !== '0' && truncated !== '1') return null;
  if (encodedCommand === undefined || encodedCommand.length > MAX_OSC_PAYLOAD_LENGTH) return null;

  try {
    const command = decodeURIComponent(encodedCommand);
    if (
      !command.trim() ||
      command.length > MAX_COMMAND_LENGTH ||
      hasUnsafeTerminalCharacters(command)
    ) return null;
    return { command, exitCode, truncated: truncated === '1' };
  } catch {
    return null;
  }
}
