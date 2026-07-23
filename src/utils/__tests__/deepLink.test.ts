import { describe, expect, it } from 'vitest';
import { parseExternalTerminalUrl } from '../deepLink';

const base64Url = (value: string): string => {
  const bytes = new TextEncoder().encode(value);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return globalThis.btoa(binary).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
};

const requestUrl = (command: string, workingDirectory = '/Users/example/My Project') =>
  `ph7console://new?cwd64=${base64Url(workingDirectory)}&command64=${base64Url(command)}`;

describe('parseExternalTerminalUrl', () => {
  it('decodes printable UTF-8 text for explicit review', () => {
    expect(parseExternalTerminalUrl(requestUrl('git status --short', '/tmp/Project ✓'))).toEqual({
      command: 'git status --short',
      workingDirectory: '/tmp/Project ✓',
    });
  });

  it.each([
    'touch /tmp/command-ran\r',
    'touch /tmp/command-ran\n',
    'printf owned\u001b[6n',
    'git\tstatus',
    'echo\u007fsecret',
    'echo\u0085secret',
    'echo safe\u202eexe.txt',
  ])('rejects terminal control characters in a command', command => {
    expect(parseExternalTerminalUrl(requestUrl(command))).toBeNull();
  });

  it('rejects control characters in a working directory', () => {
    expect(parseExternalTerminalUrl(requestUrl('pwd', '/tmp/work\nnext'))).toBeNull();
  });

  it('rejects invalid UTF-8 instead of replacing it', () => {
    expect(parseExternalTerminalUrl('ph7console://new?command64=wyg')).toBeNull();
  });

  it.each([
    'https://new?command64=ZWNobyBubw',
    'ph7console://other?command64=ZWNobyBubw',
    'ph7console://user@new?command64=ZWNobyBubw',
    'ph7console://new/execute?command64=ZWNobyBubw',
    'ph7console://new?command64=ZWNobyBubw&command64=ZWNobyB5ZXM',
    'ph7console://new?command64=ZWNobyBubw&unknown=1',
  ])('rejects an unexpected URL shape: %s', value => {
    expect(parseExternalTerminalUrl(value)).toBeNull();
  });

  it('allows an empty new-terminal request without command input', () => {
    expect(parseExternalTerminalUrl('ph7console://new')).toEqual({
      command: undefined,
      workingDirectory: undefined,
    });
  });

  it('rejects oversized decoded commands', () => {
    expect(parseExternalTerminalUrl(requestUrl('x'.repeat(8 * 1024 + 1)))).toBeNull();
  });
});
