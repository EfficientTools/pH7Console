export const quoteShellArgument = (value: string): string =>
  `'${value.replace(/'/g, "'\\''")}'`;
