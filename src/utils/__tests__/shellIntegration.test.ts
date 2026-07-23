import { afterEach, describe, expect, it, vi } from 'vitest';
import { parseShellCommandEvent } from '../shellIntegration';

const commandEndPayload = (
  command: string,
  options: { status?: string; truncated?: string } = {}
) =>
  [
    'pH7',
    'event=command_end',
    `status=${options.status ?? '0'}`,
    `truncated=${options.truncated ?? '0'}`,
    `command=${encodeURIComponent(command)}`,
  ].join(';');

describe('parseShellCommandEvent', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('decodes a valid command without treating encoded delimiters as fields', () => {
    const command = "printf '%s;%s=%s' 'hello world' '$HOME' '$(whoami)'";

    expect(parseShellCommandEvent(commandEndPayload(command))).toEqual({
      command,
      exitCode: 0,
      truncated: false,
    });
  });

  it.each([
    ['0', '0', 0, false],
    ['1', '1', 1, true],
    ['127', '0', 127, false],
    ['255', '1', 255, true],
  ])(
    'parses status %s and truncated %s',
    (status, truncated, exitCode, expectedTruncated) => {
      expect(
        parseShellCommandEvent(commandEndPayload('exit', { status, truncated }))
      ).toEqual({
        command: 'exit',
        exitCode,
        truncated: expectedTruncated,
      });
    }
  );

  it.each([
    ['', 'empty payload'],
    ['pH7', 'missing fields'],
    ['pH7;event=command_end;status=0;truncated=0;broken', 'field without equals'],
    ['pH7;event=command_start;status=0;truncated=0;command=echo', 'wrong event'],
    ['pH7;event=command_end;status=-1;truncated=0;command=echo', 'negative status'],
    ['pH7;event=command_end;status=256;truncated=0;command=echo', 'status above byte range'],
    ['pH7;event=command_end;status=ok;truncated=0;command=echo', 'non-numeric status'],
    ['pH7;event=command_end;status=0;truncated=true;command=echo', 'invalid truncation flag'],
    ['pH7;event=command_end;status=0;truncated=0;command=%E0%A4%A', 'invalid URI encoding'],
    ['pH7;event=command_end;status=0;truncated=0;command=%20%20', 'blank command'],
    ['pH7;event=command_end;status=0;truncated=0;command=echo%00oops', 'NUL byte'],
    ['pH7;event=command_end;status=0;truncated=0;command=echo%0Aowned', 'newline'],
    ['pH7;event=command_end;status=0;truncated=0;command=echo%1B%5B6n', 'terminal escape'],
    ['pH7;event=command_end;status=0;truncated=0;command=echo%E2%80%AEtxt', 'bidi override'],
  ])('rejects %s (%s)', (payload) => {
    expect(parseShellCommandEvent(payload)).toBeNull();
  });

  it('rejects duplicate fields', () => {
    expect(
      parseShellCommandEvent(
        'pH7;event=command_end;status=0;status=1;truncated=0;command=echo'
      )
    ).toBeNull();
  });

  it.each([
    '133;A',
    'iTerm2;event=command_end;status=0;truncated=0;command=echo',
    'pH8;event=command_end;status=0;truncated=0;command=echo',
  ])('ignores foreign OSC payload %s', (payload) => {
    expect(parseShellCommandEvent(payload)).toBeNull();
  });

  it('rejects oversized protocol payloads and decoded commands', () => {
    expect(parseShellCommandEvent('x'.repeat(48 * 1024 + 1))).toBeNull();
    expect(parseShellCommandEvent(commandEndPayload('x'.repeat(8 * 1024 + 1)))).toBeNull();
  });

  it('keeps credential-like and substitution text inert', () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    const command =
      'curl -H "Authorization: Bearer sk-live-example" "https://example.invalid/$(whoami)"';

    expect(parseShellCommandEvent(commandEndPayload(command))).toEqual({
      command,
      exitCode: 0,
      truncated: false,
    });
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
