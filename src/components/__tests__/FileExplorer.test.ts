import { describe, expect, it } from 'vitest';
import { quoteShellArgument } from '../../utils/shell';

describe('quoteShellArgument', () => {
  it('quotes spaces and shell substitutions as literal text', () => {
    expect(quoteShellArgument('report $(touch unsafe).txt')).toBe(
      "'report $(touch unsafe).txt'"
    );
  });

  it('escapes embedded single quotes without ending the shell argument', () => {
    expect(quoteShellArgument("Pierre's file.txt")).toBe(
      "'Pierre'\\''s file.txt'"
    );
  });
});
