import { chromium } from '@playwright/test';
import { mkdir, readdir, unlink } from 'node:fs/promises';
import path from 'node:path';

const baseUrl = process.env.PH7_STORE_CAPTURE_URL ?? 'http://127.0.0.1:5173';
const outputDirectory = path.resolve('app-store/raw-screenshots');

await mkdir(outputDirectory, { recursive: true });
const expectedCaptureNames = new Set([
  '01-private-local-assistance.png',
  '02-local-error-fix.png',
  '03-workspace-explorer.png',
  '04-searchable-history.png',
  '05-privacy-settings.png',
]);
for (const filename of await readdir(outputDirectory)) {
  if (filename.endsWith('.png') && !expectedCaptureNames.has(filename)) {
    await unlink(path.join(outputDirectory, filename));
  }
}

const browser = await chromium.launch({
  // xterm's DOM/canvas renderer is deterministic in headless captures. The
  // WebGL addon remains exercised in the native app, but SwiftShader can
  // produce an empty terminal surface in headless Chromium screenshots.
  args: ['--force-color-profile=srgb', '--disable-webgl'],
});
const page = await browser.newPage({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 2,
  colorScheme: 'dark',
  locale: 'en-US',
  timezoneId: 'Australia/Sydney',
});

// Keep relative history times and suggestion timestamps repeatable without
// replacing browser timers, requestAnimationFrame, or xterm's render loop.
await page.clock.setFixedTime(new Date('2026-07-18T00:00:00.000Z'));

await page.addInitScript(() => {
  const workspace = '/Users/developer/Projects/pH7Console';
  const fixedNow = new Date('2026-07-18T00:00:00.000Z').getTime();
  localStorage.setItem('ph7console-settings', JSON.stringify({
    state: {
      // Chromium's generic `ui-monospace` metrics differ from WebKit's and can
      // exaggerate xterm cell spacing in deterministic marketing captures.
      // Use the same native macOS fonts users can select in the app.
      appearance: {
        fontSize: 14,
        fontFamily: 'SFMono-Regular, Menlo, Monaco, monospace',
      },
      keyboard: {
        shortcuts: {
          newTerminal: '⌘T',
          closeTerminal: '⌘W',
          toggleAI: '⌘J',
          clearTerminal: '⌘K',
        },
      },
    },
    version: 0,
  }));

  const history = [
    {
      id: 'history-git-status',
      session_id: 'session-main-workspace',
      command: 'git status --short',
      output: '',
      exit_code: 0,
      duration_ms: 42,
      timestamp: new Date(fixedNow - 18 * 60_000).toISOString(),
      working_directory: workspace,
    },
    {
      id: 'history-shell-parser-tests',
      session_id: 'session-main-workspace',
      command: 'npm run test:unit -- shellIntegration',
      output: '',
      exit_code: 0,
      duration_ms: 812,
      timestamp: new Date(fixedNow - 12 * 60_000).toISOString(),
      working_directory: workspace,
    },
    {
      id: 'history-shell-tests',
      session_id: 'session-main-workspace',
      command: 'cargo test shell_integration',
      output: '',
      exit_code: 0,
      duration_ms: 1160,
      timestamp: new Date(fixedNow - 8 * 60_000).toISOString(),
      working_directory: workspace,
    },
    {
      id: 'history-production-build',
      session_id: 'session-main-workspace',
      command: 'npm run build',
      output: '',
      exit_code: 0,
      duration_ms: 1320,
      timestamp: new Date(fixedNow - 4 * 60_000).toISOString(),
      working_directory: workspace,
    },
    {
      id: 'history-build-typo',
      session_id: 'session-build-diagnostics',
      command: 'npm run bild',
      output: '',
      exit_code: 1,
      duration_ms: 126,
      timestamp: new Date(fixedNow - 2 * 60_000).toISOString(),
      working_directory: workspace,
    },
  ];

  const mainSnapshot = [
    '\u001b]7;file://localhost/Users/developer/Projects/pH7Console\u0007',
    '\u001b[38;5;111m~/pH7Console\u001b[0m \u001b[38;5;141m(main)\u001b[0m \u001b[1m❯\u001b[0m git status --short\r\n',
    ' \u001b[33mM\u001b[0m src/components/Terminal.tsx\r\n',
    ' \u001b[33mM\u001b[0m src-tauri/src/terminal/mod.rs\r\n',
    ' \u001b[32mA\u001b[0m src-tauri/src/shell_integration.rs\r\n',
    '\r\n',
    '\u001b[38;5;111m~/pH7Console\u001b[0m \u001b[38;5;141m(main)\u001b[0m \u001b[1m❯\u001b[0m npm run test:unit -- shellIntegration\r\n',
    '\u001b[2m RUN  v3.2.4 /Users/developer/Projects/pH7Console\u001b[0m\r\n',
    '\u001b[32m ✓\u001b[0m src/utils/__tests__/shellIntegration.test.ts \u001b[2m(22 tests)\u001b[0m\r\n',
    '\u001b[1;32m Test Files  1 passed (1)\u001b[0m\r\n',
    '\u001b[1;32m      Tests  22 passed (22)\u001b[0m\r\n',
    '\u001b[2m   Duration  812ms\u001b[0m\r\n',
    '\r\n',
    '\u001b[38;5;111m~/pH7Console\u001b[0m \u001b[38;5;141m(main)\u001b[0m \u001b[1m❯\u001b[0m cargo test shell_integration\r\n',
    '\u001b[32m    Finished\u001b[0m test profile in 0.18s\r\n',
    '\u001b[32m     Running\u001b[0m 9 tests\r\n',
    '\u001b[1;32mtest result: ok. 9 passed; 0 failed; 0 ignored\u001b[0m\r\n',
    '\r\n',
    '\u001b[38;5;111m~/pH7Console\u001b[0m \u001b[38;5;141m(main)\u001b[0m \u001b[1m❯\u001b[0m ',
  ].join('');

  const diagnosticsSnapshot = [
    '\u001b]7;file://localhost/Users/developer/Projects/pH7Console\u0007',
    '\u001b[38;5;111m~/pH7Console\u001b[0m \u001b[38;5;141m(main)\u001b[0m \u001b[1m❯\u001b[0m npm run bild\r\n',
    '\r\n',
    '\u001b[1;31mnpm error Missing script: "bild"\u001b[0m\r\n',
    '\u001b[33mnpm error Did you mean this?\u001b[0m\r\n',
    '\u001b[1m  npm run build\u001b[0m\r\n',
    '\r\n',
    '\u001b[2mTo see a list of scripts, run:\u001b[0m\r\n',
    '\u001b[2m  npm run\u001b[0m\r\n',
    '\r\n',
    '\u001b[38;5;111m~/pH7Console\u001b[0m \u001b[38;5;141m(main)\u001b[0m \u001b[1m❯\u001b[0m ',
  ].join('');

  const callbacks = new Map();
  const sessions = [];
  let nextCallbackId = 1;
  let nextEventId = 1;
  let nextSubscriberId = 1;

  const encodeBase64 = (value) => {
    const bytes = new TextEncoder().encode(value);
    let binary = '';
    for (const byte of bytes) binary += String.fromCharCode(byte);
    return window.btoa(binary);
  };

  const encodeStreamFrame = (sequence, value = '') => {
    const body = new TextEncoder().encode(value);
    const frame = new Uint8Array(9 + body.length);
    frame[0] = 1;
    let remaining = sequence;
    for (let index = 8; index >= 1; index -= 1) {
      frame[index] = remaining & 0xff;
      remaining = Math.floor(remaining / 256);
    }
    frame.set(body, 9);
    return frame;
  };

  const slug = (value) => value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '');

  const directoryChildren = (currentPath) => {
    if (currentPath === workspace) {
      return [
        { name: 'src', path: `${workspace}/src`, is_directory: true },
        { name: 'src-tauri', path: `${workspace}/src-tauri`, is_directory: true },
        { name: 'scripts', path: `${workspace}/scripts`, is_directory: true },
        { name: 'tests', path: `${workspace}/tests`, is_directory: true },
        { name: 'README.md', path: `${workspace}/README.md`, is_directory: false },
        { name: 'package.json', path: `${workspace}/package.json`, is_directory: false },
        { name: 'Cargo.toml', path: `${workspace}/src-tauri/Cargo.toml`, is_directory: false },
      ];
    }
    if (currentPath === `${workspace}/src`) {
      return [
        { name: 'components', path: `${currentPath}/components`, is_directory: true },
        { name: 'store', path: `${currentPath}/store`, is_directory: true },
        { name: 'utils', path: `${currentPath}/utils`, is_directory: true },
        { name: 'App.tsx', path: `${currentPath}/App.tsx`, is_directory: false },
        { name: 'main.tsx', path: `${currentPath}/main.tsx`, is_directory: false },
        { name: 'index.css', path: `${currentPath}/index.css`, is_directory: false },
      ];
    }
    if (currentPath === `${workspace}/src-tauri`) {
      return [
        { name: 'src', path: `${currentPath}/src`, is_directory: true },
        { name: 'binaries', path: `${currentPath}/binaries`, is_directory: true },
        { name: 'Cargo.toml', path: `${currentPath}/Cargo.toml`, is_directory: false },
        { name: 'tauri.conf.json', path: `${currentPath}/tauri.conf.json`, is_directory: false },
      ];
    }
    return [];
  };

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: () => {},
  };

  window.__TAURI_INTERNALS__ = {
    invoke: async (command, args = {}) => {
      switch (command) {
        case 'initialize_ml_system':
          return 'Private local command intelligence ready';
        case 'get_local_llm_status':
          return {
            available: true,
            backend: 'llama.cpp (bundled)',
            models: ['Qwen2.5-Coder-1.5B-Instruct'],
            message: 'Verified loopback-only local LLM ready',
          };
        case 'get_voice_input_status':
          return {
            kind: 'status',
            available: true,
            onDeviceAvailable: true,
            microphoneAuthorization: 'authorized',
            speechAuthorization: 'authorized',
            message: 'Ready for on-device voice input',
          };
        case 'create_terminal': {
          const title = String(args.title ?? `Terminal ${sessions.length + 1}`);
          const id = sessions.length === 0
            ? 'session-main-workspace'
            : title === 'Terminal 2'
              ? 'session-build-diagnostics'
              : `session-${slug(title)}`;
          const displayTitle = id === 'session-main-workspace'
            ? 'Main workspace'
            : id === 'session-build-diagnostics'
              ? 'Build diagnostics'
              : title;
          const existing = sessions.find((session) => session.id === id);
          if (!existing) {
            sessions.push({
              id,
              title: displayTitle,
              working_directory: workspace,
              is_active: true,
              created_at: new Date(fixedNow - sessions.length * 60_000).toISOString(),
            });
          }
          return id;
        }
        case 'get_all_sessions':
          return sessions.map((session) => ({ ...session }));
        case 'change_directory': {
          const session = sessions.find((item) => item.id === args.sessionId);
          if (session) session.working_directory = String(args.newPath);
          return String(args.newPath);
        }
        case 'sync_terminal_working_directory':
          return String(args.workingDirectory ?? workspace);
        case 'attach_terminal_stream': {
          const subscriberId = nextSubscriberId;
          nextSubscriberId += 1;
          // Exercise the ordered binary channel after the snapshot without
          // adding staged marketing text to the actual terminal output.
          window.setTimeout(() => {
            args.onEvent?.onmessage?.(encodeStreamFrame(41));
          }, 30);
          return subscriberId;
        }
        case 'get_terminal_snapshot': {
          const value = String(args.sessionId).includes('build-diagnostics')
            ? diagnosticsSnapshot
            : mainSnapshot;
          return {
            dataBase64: encodeBase64(value),
            lastSequence: 40,
            isRunning: true,
            processId: 4242,
          };
        }
        case 'detach_terminal_stream':
        case 'resize_terminal':
        case 'write_to_terminal':
        case 'write_bytes_to_terminal':
        case 'close_terminal_session':
        case 'persist_terminal_sessions':
        case 'store_command_in_history':
        case 'record_shell_command':
        case 'update_ai_feedback':
        case 'file_action':
          return null;
        case 'update_session_title': {
          const session = sessions.find((item) => item.id === args.sessionId);
          if (session) session.title = String(args.title);
          return null;
        }
        case 'get_recent_command_history':
          return history.map((item) => ({ ...item }));
        case 'get_history_persistence_status':
          return {
            encryptedPersistence: true,
            mode: 'encrypted',
            message: 'Encrypted on this Mac',
          };
        case 'clear_command_history':
          return null;
        case 'get_command_history_for_navigation':
          return history.map((item) => item.command).reverse();
        case 'search_command_history_records': {
          const query = String(args.query ?? '').toLowerCase();
          return history
            .filter((item) => item.command.toLowerCase().includes(query))
            .slice()
            .reverse()
            .map((item) => ({ ...item }));
        }
        case 'get_repo_info':
          return {
            repo_name: 'pH7Console',
            current_branch: 'main',
            has_changes: true,
            ahead: 0,
            behind: 0,
            remote_url: 'github.com/EfficientTools/pH7Console',
            is_git_repo: true,
          };
        case 'get_runtime_info':
          return {
            node_version: 'v22.18.0',
            npm_version: '10.9.3',
            rust_version: '1.88.0',
            python_version: '3.12.11',
            git_version: '2.50.1',
            go_version: null,
            java_version: null,
            project_type: 'typescript',
          };
        case 'get_parent_directories':
          return [
            { name: 'developer', path: '/Users/developer', is_directory: true },
            { name: 'Projects', path: '/Users/developer/Projects', is_directory: true },
          ];
        case 'get_child_directories':
          return directoryChildren(String(args.currentPath));
        case 'get_path_completions':
          return [];
        case 'get_smart_completions':
          return [];
        case 'create_command_plan':
          if (String(args.input).includes('diagnose or fix')) {
            return {
              command: 'npm run build',
              confidence: 0.92,
              explanation: 'Corrects the misspelled package script. Review the command before inserting it; nothing runs automatically.',
              source: 'local_llm',
              riskLevel: 'low',
              riskReasons: ['Runs the project build script', 'Does not elevate privileges'],
              requiresConfirmation: false,
              requiresStrongConfirmation: false,
            };
          }
          return {
            command: 'find . -type f -size +100M -print',
            confidence: 0.97,
            explanation: 'Scans only the selected workspace and lists files larger than 100 MB. Nothing runs until you insert and execute it.',
            source: 'local_llm',
            riskLevel: 'low',
            riskReasons: ['Read-only workspace scan', 'No files are modified'],
            requiresConfirmation: false,
            requiresStrongConfirmation: false,
          };
        case 'ai_suggest_command':
          return {
            text: 'git diff --stat',
            confidence: 0.94,
            reasoning: 'Based on the selected repository and recent local commands.',
          };
        case 'ai_explain_command':
          return {
            text: 'Shows tracked and untracked repository changes in a compact format.',
            confidence: 0.97,
          };
        case 'ai_fix_error':
          return {
            text: 'Use npm run build. The package script was misspelled as “bild”; review the corrected command before running it.',
            confidence: 0.99,
          };
        case 'ai_analyze_output':
          return {
            text: 'The selected test suite completed successfully with no failed cases.',
            confidence: 0.98,
          };
        case 'get_user_analytics':
          return {
            total_commands: 148,
            success_rate: 0.94,
            most_used_commands: [['git status', 18], ['npm test', 12]],
            learning_examples: 26,
            patterns_learned: 9,
          };
        case 'plugin:deep-link|get_current':
          return null;
        case 'plugin:event|listen': {
          const eventId = nextEventId;
          nextEventId += 1;
          return eventId;
        }
        case 'plugin:event|unlisten':
          return null;
        default:
          throw new Error(`Unhandled store capture command: ${command}`);
      }
    },
    transformCallback: (callback, once = false) => {
      const id = nextCallbackId;
      nextCallbackId += 1;
      callbacks.set(id, { callback, once });
      return id;
    },
    unregisterCallback: (id) => {
      callbacks.delete(id);
    },
    convertFileSrc: (value) => value,
  };
});

const pageErrors = [];
page.on('pageerror', error => pageErrors.push(error));

const settle = async () => {
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(180);
};

const capture = async (filename) => {
  await settle();
  await page.screenshot({
    path: path.join(outputDirectory, filename),
    animations: 'disabled',
    caret: 'hide',
    scale: 'device',
  });
};

try {
  await page.goto(baseUrl, { waitUntil: 'networkidle' });
  await page.getByText('Local LLM', { exact: true }).waitFor();
  await page.getByText('Live PTY', { exact: true }).first().waitFor();
  // Multiple sessions remain mounted so their PTYs never restart. Wait for
  // xterm construction here; the active session is selected explicitly below.
  await page.locator('.xterm-screen').first().waitFor({ state: 'attached' });
  await page.getByRole('button', { name: 'New terminal' }).click();
  if (process.env.PH7_CAPTURE_DEBUG === '1') {
    await page.waitForTimeout(500);
    console.log(await page.locator('body').innerText());
  }
  await page.getByRole('button', { name: /^Build diagnostics,/ }).waitFor();
  await page.getByRole('button', { name: /^Main workspace,/ }).click();

  const commandPrompt = page.getByRole('textbox', { name: 'Describe a command to plan' });
  await commandPrompt.fill('Show files larger than 100 MB in this workspace');
  await page.getByRole('button', { name: 'Create Safe Command Plan' }).click();
  await page.getByText('find . -type f -size +100M -print', { exact: true }).waitFor();
  await page.getByText('low', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Insert command without executing' }).waitFor();
  // Keep the plain-English request visible beside the completed local plan so
  // the first store image communicates the input-to-command workflow at once.
  await commandPrompt.fill('Show files larger than 100 MB in this workspace');
  await capture('01-private-local-assistance.png');

  await page.getByRole('button', { name: /^Build diagnostics,/ }).click();
  await page.locator('#ph7-ai-panel').getByRole('button', { name: 'Clear', exact: true }).click();
  await commandPrompt.fill('Diagnose or fix the last failed command');
  await page.getByRole('button', { name: 'Fix Error' }).click();
  await page.locator('#ph7-ai-panel').getByText('npm run build', { exact: true }).waitFor();
  await page.locator('#ph7-ai-panel').getByText('On-device model', { exact: true }).first().waitFor();
  await capture('02-local-error-fix.png');

  await page.getByRole('button', { name: /^Main workspace,/ }).click();
  await page.locator('#ph7-ai-panel').getByRole('button', { name: 'Clear', exact: true }).click();
  await commandPrompt.fill('Show files larger than 100 MB in this workspace');
  await page.getByRole('button', { name: 'Create Safe Command Plan' }).click();
  await page.getByText('find . -type f -size +100M -print', { exact: true }).waitFor();
  await commandPrompt.fill('Show files larger than 100 MB in this workspace');
  await page.getByRole('tab', { name: 'Explorer' }).click();
  await page.getByText('README.md', { exact: true }).waitFor();
  await page.getByText('src', { exact: true }).click();
  await page.getByText('App.tsx', { exact: true }).waitFor();
  await capture('03-workspace-explorer.png');

  await page.getByRole('tab', { name: 'Terminals' }).click();
  await page.getByRole('button', { name: 'Open command history' }).click();
  await page.getByRole('heading', { name: 'Command History' }).waitFor();
  await page.getByRole('textbox', { name: 'Search command history' }).fill('test');
  await page.getByText('2 results', { exact: false }).waitFor();
  await capture('04-searchable-history.png');

  await page.getByRole('button', { name: 'Close command history' }).click();
  await page.getByRole('button', { name: 'Settings' }).click();
  await page.getByRole('heading', { name: 'Settings' }).waitFor();
  await page.getByRole('tab', { name: 'Privacy' }).click();
  await page.getByRole('heading', { name: 'Privacy' }).waitFor();
  await page.getByText('Encrypted on this Mac', { exact: true }).waitFor();
  await capture('05-privacy-settings.png');

  if (pageErrors.length > 0) {
    throw new AggregateError(pageErrors, 'The capture page raised browser errors');
  }
} finally {
  await browser.close();
}

console.log(`Captured 5 truthful 2880x1800 App Store source screenshots in ${outputDirectory}`);
