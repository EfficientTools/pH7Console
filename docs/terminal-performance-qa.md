# Terminal performance and compatibility QA

`scripts/terminal_qa.py` is a dependency-free, non-destructive harness for
terminal performance work. It has two deliberately separate roles:

1. `selftest` measures the host operating system's PTY floor. This catches slow
   shell configuration, broken signal delivery, resize failures, output
   corruption, and test-machine regressions without depending on pH7Console.
2. `workload` emits repeatable stress streams inside pH7Console. These exercise
   the complete PTY reader, Tauri channel, buffering, xterm renderer, input,
   resize, and tab-isolation path.

The host floor is not an application benchmark. A passing `selftest` means the
machine is fit to test; the end-to-end gates below determine whether pH7Console
is release-ready.

## Quick start

Run the automated host checks from the repository root:

```bash
python3 scripts/terminal_qa.py selftest
```

Save a report for CI or a QA attachment:

```bash
python3 scripts/terminal_qa.py selftest --json-stdout
python3 scripts/terminal_qa.py selftest --output-json /tmp/ph7-terminal-qa.json
```

Every workload writes only to its own standard output and standard error. The
harness does not read project files, change shell configuration, install
software, make network calls, or persist anything unless `--output-json` is
explicitly supplied.

## End-to-end workload recipes

Run these commands in pH7Console, not in the host terminal used to launch the
app. Keep the window at 120 columns by 36 rows, use the same font and scrollback
settings for every comparison, and wait 30 seconds after launch before warm
runs.

### Bulk output

```bash
python3 scripts/terminal_qa.py workload bulk --bytes 104857600 --chunk-size 65536
```

The final `PH7QA_RESULT` line reports generator time, throughput, byte count,
and SHA-256. Generator throughput is a lower-level diagnostic; measure wall
time until the prompt is interactive again for the end-to-end result. The tab
must remain selectable and Ctrl-C must remain responsive throughout.

### Fragmented output

```bash
python3 scripts/terminal_qa.py workload fragmented --bytes 16777216 --fragment-size 64
```

This stresses read aggregation and IPC overhead. Repeat once with
`--fragment-size 1` as an adversarial soak test; the one-byte case is not a
release throughput gate.

### ANSI and TUI behavior

```bash
python3 scripts/terminal_qa.py workload ansi --frames 600 --rows 36 --columns 120 --delay-ms 16.667
```

Verify that alternate-screen entry and exit are clean, the cursor returns, the
title updates do not corrupt output, truecolor gradients render, and the OSC 8
label is link-aware. There must be no flashes from stale frames after changing
tabs.

### Unicode and grapheme handling

```bash
python3 scripts/terminal_qa.py workload unicode --lines 10000
```

Inspect combining marks, CJK width, RTL samples, emoji/ZWJ clusters, flags,
variation selectors, and box drawing. Search and selection must preserve the
original UTF-8 text with no replacement characters.

### Ctrl-C

```bash
python3 scripts/terminal_qa.py workload interrupt --interval 0.02
```

Press Ctrl-C. The workload prints `PH7QA_INTERRUPT_HANDLED` when the child
receives SIGINT. Repeat 30 times. For precise latency, use a 120 fps screen
recording or application signposts and measure from key-down to acknowledgement;
the host `selftest` provides the PTY-only control result.

### Resize

```bash
python3 scripts/terminal_qa.py workload resize --duration 60
```

Drag continuously between narrow/wide and short/tall sizes. Each delivered
SIGWINCH reports the child-visible rows and columns. Compare the final report to
the xterm grid; the last event must always match, including after fullscreen and
tab switches.

### Cross-session isolation

Open two tabs. In tab A run:

```bash
python3 scripts/terminal_qa.py workload fragmented --bytes 268435456 --fragment-size 32
```

In tab B run:

```bash
python3 scripts/terminal_qa.py workload echo --duration 120
```

Enter at least 30 unique lines in tab B while tab A floods. Every line must
receive exactly one `PH7QA_ECHO` response, no `PH7QA_BULK_PAYLOAD` bytes may
appear in tab B, tab switching must stay immediate, and Ctrl-C in either tab
must affect only that tab.

### Process-tree memory

Find the pH7Console application PID in Activity Monitor, then run from a host
terminal:

```bash
python3 scripts/terminal_qa.py memory --pid APP_PID --seconds 60 --include-children --max-mib 450
```

`--include-children` is important on macOS because WebKit content processes can
hold terminal buffers. Record idle, ten-tab idle, active flood, 30 seconds after
the flood, and 30 seconds after closing the flood tab.

## Release acceptance gates

Test on the oldest supported macOS version and a fanless Apple-silicon MacBook
Air with a normal user shell configuration. Use at least 10 samples for startup
and 30 for input/signal latency. Report median, p95, maximum, machine, OS, build,
shell, font, window grid, and scrollback size.

| Area | Required gate |
| --- | --- |
| Cold app startup | First visible, interactive prompt: median <= 1,000 ms and p95 <= 1,500 ms. |
| Warm tab startup | New persistent shell accepts input: median <= 250 ms and p95 <= 500 ms; application overhead should be <= host shell p95 + 150 ms. |
| Bulk output | 100 MiB completes end-to-end in <= 4.0 s (>= 25 MiB/s), no lost/truncated tail marker, and UI controls remain usable. |
| Fragmented output | 16 MiB in 64-byte writes sustains >= 3 MiB/s, returns the exact tail marker, and does not grow an unbounded event queue. |
| ANSI/TUI | 600 frames at 60 Hz complete without stale frames, alternate-screen leakage, cursor loss, or any interaction stall over 100 ms. |
| Unicode | Zero invalid UTF-8 or replacement characters; grapheme width, selection, copy, and search remain correct for every supplied case. |
| Ctrl-C | Key-down to child SIGINT acknowledgement: median <= 50 ms, p95 <= 100 ms, maximum <= 200 ms, zero misses in 30 attempts. |
| Resize | UI resize to child-visible `TIOCGWINSZ`: p95 <= 100 ms, maximum <= 250 ms, and the final grid is exact. |
| Session isolation | During the 256 MiB flood, other-tab echo p95 <= 75 ms and maximum <= 150 ms; zero cross-tab bytes, signals, cwd, or history leakage. |
| Idle memory | Full process tree <= 225 MiB with one tab and <= 400 MiB with ten idle tabs. |
| Flood memory | Full process tree <= 450 MiB after output settles; closing the flood tab returns within 10% of its pre-flood baseline within 30 seconds. |
| Reliability soak | Eight hours, 100 tab create/close cycles, 10 GiB aggregate output, and repeated sleep/wake: zero crashes, deadlocks, orphan shells, or lost final output. |

A release fails if any absolute gate fails. Do not average away a signal miss,
cross-session leak, crash, orphaned process, or corrupted byte stream.

## Fair competitor comparison

Use the exact same machine, account, shell startup files, working directory,
font size, window grid, scrollback, foreground/background state, and workload
command. Alternate products between runs (A/B/B/A) to reduce thermal and cache
bias. Discard warm-up runs, retain raw samples, and compare p50 and p95 rather
than only the best result.

Only describe a metric as a performance win when pH7Console passes every
absolute gate, improves the competitor median by at least 10% on that metric,
and is not more than 10% worse at p95 on any reliability-critical latency.
Privacy, offline capability, and local reasoning are product advantages, but
they should not be presented as substitutes for measured terminal performance.

## Reading failures

- A slow `host_shell_startup_p95` usually points to shell startup files or a
  prompt plugin. Compare app startup against that measured floor.
- Fast host bulk output with slow in-app bulk output points to PTY reading, IPC
  batching, xterm writes, or render scheduling.
- Fast bulk but slow fragmented output points to per-read/per-message overhead.
- Slow cross-session echo with a passing host isolation test points to shared
  locks, a single event queue, or main-thread rendering starvation.
- A passing resize marker with visual corruption points to renderer layout;
  missing or stale markers point to PTY resize propagation.
- Rising RSS after closing the flood tab points to retained xterm buffers,
  listeners, channels, or backend session state.

The JSON self-test report is intentionally stable (`schema_version: 1`) so it
can be attached to release artifacts or consumed by a future CI job without
coupling the harness to application code.
