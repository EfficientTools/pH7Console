#!/usr/bin/env python3
"""Portable, dependency-free PTY performance and compatibility QA harness.

The automated checks measure the host PTY floor.  The workload generators are
also intended to be run inside pH7Console so the complete renderer/IPC path can
be observed without importing or modifying application internals.
"""

from __future__ import annotations

import argparse
import errno
import fcntl
import hashlib
import json
import math
import os
import platform
import pty
import select
import signal
import statistics
import struct
import subprocess
import sys
import termios
import threading
import time
from pathlib import Path
from typing import Any, Iterable, Optional


MIB = 1024 * 1024
RESULT_PREFIX = b"PH7QA_RESULT "
READY_INTERRUPT = b"PH7QA_INTERRUPT_READY"
HANDLED_INTERRUPT = b"PH7QA_INTERRUPT_HANDLED"
READY_ECHO = b"PH7QA_ECHO_READY"
BULK_PATTERN = (
    b"PH7QA_BULK_PAYLOAD_0123456789abcdef_"
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz\n"
)


def _positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return parsed


def _non_negative_float(value: str) -> float:
    parsed = float(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("must be zero or greater")
    return parsed


def _positive_float(value: str) -> float:
    parsed = float(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return parsed


def _write_all(fd: int, data: bytes) -> None:
    view = memoryview(data)
    while view:
        try:
            written = os.write(fd, view)
        except InterruptedError:
            continue
        if written == 0:
            raise BrokenPipeError("output closed")
        view = view[written:]


def _pattern_slice(offset: int, length: int) -> bytes:
    start = offset % len(BULK_PATTERN)
    repeats = math.ceil((start + length) / len(BULK_PATTERN))
    return (BULK_PATTERN * max(1, repeats))[start : start + length]


def _emit_result(kind: str, **fields: Any) -> None:
    payload = {"kind": kind, **fields}
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    _write_all(2, b"\n" + RESULT_PREFIX + encoded + b"\n")


def _emit_bytes(total: int, write_size: int, delay_seconds: float = 0.0) -> dict[str, Any]:
    digest = hashlib.sha256()
    emitted = 0
    started = time.perf_counter()
    while emitted < total:
        size = min(write_size, total - emitted)
        chunk = _pattern_slice(emitted, size)
        _write_all(1, chunk)
        digest.update(chunk)
        emitted += size
        if delay_seconds:
            time.sleep(delay_seconds)
    elapsed = max(time.perf_counter() - started, 1e-9)
    return {
        "bytes": emitted,
        "seconds": round(elapsed, 6),
        "mib_per_second": round((emitted / MIB) / elapsed, 3),
        "sha256": digest.hexdigest(),
    }


def workload_bulk(args: argparse.Namespace) -> int:
    result = _emit_bytes(args.bytes, args.chunk_size)
    _emit_result("bulk", chunk_size=args.chunk_size, **result)
    return 0


def workload_fragmented(args: argparse.Namespace) -> int:
    result = _emit_bytes(args.bytes, args.fragment_size, args.delay_us / 1_000_000)
    _emit_result(
        "fragmented",
        fragment_size=args.fragment_size,
        delay_us=args.delay_us,
        **result,
    )
    return 0


def workload_ansi(args: argparse.Namespace) -> int:
    rows = min(args.rows, 200)
    columns = min(args.columns, 500)
    emitted = 0
    digest = hashlib.sha256()
    started = time.perf_counter()

    def send(data: bytes) -> None:
        nonlocal emitted
        _write_all(1, data)
        emitted += len(data)
        digest.update(data)

    enter = b"\x1b[?1049h\x1b[?25l\x1b[2J"
    leave = b"\x1b[0m\x1b[?25h\x1b[?1049l"
    try:
        send(enter)
        for frame in range(args.frames):
            parts = [
                b"\x1b[H",
                f"\x1b[1;38;2;126;231;135m pH7Console ANSI frame {frame + 1}/{args.frames}\x1b[0m".encode(),
            ]
            for row in range(2, rows + 1):
                red = (frame * 7 + row * 11) % 256
                green = (frame * 13 + row * 5) % 256
                blue = (frame * 3 + row * 17) % 256
                fill_width = max(8, columns - 24)
                fill = ("#" if (frame + row) % 2 else "=") * fill_width
                parts.append(
                    f"\x1b[{row};1H\x1b[38;2;{red};{green};{blue}m{row:03d} {fill}\x1b[0m".encode()
                )
            # Cursor save/restore, erase-in-line, OSC 8 hyperlink, and a title update.
            parts.extend(
                [
                    b"\x1b7",
                    f"\x1b[{rows};1H\x1b[2Kframe={frame:05d} 24-bit-color=true".encode(),
                    b" \x1b]8;;https://example.invalid/ph7qa\x1b\\OSC-8\x1b]8;;\x1b\\",
                    b"\x1b8",
                    f"\x1b]0;pH7 QA frame {frame}\x07".encode(),
                ]
            )
            send(b"".join(parts))
            if args.delay_ms:
                time.sleep(args.delay_ms / 1000)
    finally:
        send(leave)

    elapsed = max(time.perf_counter() - started, 1e-9)
    _emit_result(
        "ansi",
        frames=args.frames,
        rows=rows,
        columns=columns,
        bytes=emitted,
        seconds=round(elapsed, 6),
        frames_per_second=round(args.frames / elapsed, 3),
        sha256=digest.hexdigest(),
    )
    return 0


UNICODE_CASES = (
    "ASCII baseline: pH7Console",
    "Combining: cafe\u0301 nai\u0308ve A\u030a",
    "Precomposed: café naïve Ångström",
    "CJK: 終端性能測試 / 터미널 성능 / 端末テスト",
    "RTL: العربية עברית فارسی",
    "Emoji: 🧠⚡️ 🧑🏽‍💻 👨‍👩‍👧‍👦 🏳️‍🌈",
    "Flags and keycaps: 🇦🇺 🇫🇷 1️⃣ #️⃣",
    "Box drawing: ┌─┬─┐ │ ╳ │ └─┴─┘ ░▒▓█",
    "Math: ∀x∈ℝ, x²≥0; ∑∞ₙ₌₁ 1/n² = π²/6",
    "Zero-width joiner boundary: a‍b and variation: ☕️",
)


def workload_unicode(args: argparse.Namespace) -> int:
    digest = hashlib.sha256()
    emitted = 0
    started = time.perf_counter()
    for index in range(args.lines):
        text = f"{index:06d} {UNICODE_CASES[index % len(UNICODE_CASES)]}\n"
        encoded = text.encode("utf-8")
        _write_all(1, encoded)
        digest.update(encoded)
        emitted += len(encoded)
    elapsed = max(time.perf_counter() - started, 1e-9)
    _emit_result(
        "unicode",
        lines=args.lines,
        bytes=emitted,
        seconds=round(elapsed, 6),
        sha256=digest.hexdigest(),
    )
    return 0


def workload_interrupt(args: argparse.Namespace) -> int:
    def interrupted(_signum: int, _frame: Any) -> None:
        stamp = time.monotonic_ns()
        try:
            _write_all(1, f"\r\n{HANDLED_INTERRUPT.decode()} {stamp}\r\n".encode())
        finally:
            os._exit(130)

    signal.signal(signal.SIGINT, interrupted)
    _write_all(1, READY_INTERRUPT + b"\r\n")
    count = 0
    while True:
        if args.interval:
            _write_all(1, f"heartbeat {count:08d}\r\n".encode())
            count += 1
        time.sleep(max(args.interval, 0.01))


def _terminal_dimensions() -> tuple[int, int]:
    try:
        size = os.get_terminal_size(0)
        return size.lines, size.columns
    except OSError:
        return 0, 0


def workload_resize(args: argparse.Namespace) -> int:
    started = time.monotonic()
    events = 0

    def emit_resize(_signum: int = 0, _frame: Any = None) -> None:
        nonlocal events
        rows, columns = _terminal_dimensions()
        record = json.dumps(
            {"rows": rows, "cols": columns, "received_ns": time.monotonic_ns()},
            separators=(",", ":"),
        )
        _write_all(1, b"PH7QA_RESIZE " + record.encode() + b"\r\n")
        events += 1

    signal.signal(signal.SIGWINCH, emit_resize)
    emit_resize()
    try:
        while time.monotonic() - started < args.duration:
            time.sleep(0.05)
    except KeyboardInterrupt:
        pass
    _emit_result("resize", events=events, seconds=round(time.monotonic() - started, 6))
    return 0


def workload_echo(args: argparse.Namespace) -> int:
    _write_all(1, READY_ECHO + b"\r\n")
    deadline = time.monotonic() + args.duration
    pending = b""
    received = 0
    while time.monotonic() < deadline:
        ready, _, _ = select.select([0], [], [], min(0.1, max(0.0, deadline - time.monotonic())))
        if not ready:
            continue
        data = os.read(0, 4096)
        if not data:
            break
        pending += data
        while b"\n" in pending:
            line, pending = pending.split(b"\n", 1)
            token = line.rstrip(b"\r")[:256].decode("utf-8", "replace")
            record = json.dumps(
                {"token": token, "received_ns": time.monotonic_ns()},
                separators=(",", ":"),
            )
            _write_all(1, b"PH7QA_ECHO " + record.encode() + b"\r\n")
            received += 1
    _emit_result("echo", received=received)
    return 0


class PtyProcess:
    """Small POSIX PTY subprocess wrapper used only by the QA controller."""

    def __init__(self, argv: list[str], env: Optional[dict[str, str]] = None) -> None:
        pid, fd = pty.fork()
        if pid == 0:
            child_env = os.environ.copy()
            if env:
                child_env.update(env)
            os.execve(argv[0], argv, child_env)
        self.pid = pid
        self.fd = fd
        self.status: Optional[int] = None
        self.closed = False

    def write(self, data: bytes) -> None:
        _write_all(self.fd, data)

    def read(self, timeout: float = 0.1, size: int = 65536) -> Optional[bytes]:
        if self.closed:
            return b""
        ready, _, _ = select.select([self.fd], [], [], max(0.0, timeout))
        if not ready:
            return None
        try:
            return os.read(self.fd, size)
        except OSError as error:
            if error.errno == errno.EIO:
                return b""
            raise

    def read_until(self, needle: bytes, timeout: float, max_capture: int = 2 * MIB) -> bytes:
        deadline = time.monotonic() + timeout
        captured = bytearray()
        while time.monotonic() < deadline:
            data = self.read(min(0.1, deadline - time.monotonic()))
            if data:
                captured.extend(data)
                if len(captured) > max_capture:
                    del captured[: len(captured) - max_capture]
                if needle in captured:
                    return bytes(captured)
            elif data == b"" and self.poll() is not None:
                break
        raise TimeoutError(f"timed out waiting for {needle!r}")

    def set_echo(self, enabled: bool) -> None:
        attributes = termios.tcgetattr(self.fd)
        if enabled:
            attributes[3] |= termios.ECHO
        else:
            attributes[3] &= ~termios.ECHO
        termios.tcsetattr(self.fd, termios.TCSANOW, attributes)

    def resize(self, rows: int, columns: int) -> None:
        packed = struct.pack("HHHH", rows, columns, 0, 0)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, packed)

    def poll(self) -> Optional[int]:
        if self.status is not None:
            return self.status
        try:
            pid, status = os.waitpid(self.pid, os.WNOHANG)
        except ChildProcessError:
            return self.status if self.status is not None else 0
        if pid:
            self.status = status
        return self.status

    def wait(self, timeout: float = 2.0) -> Optional[int]:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            status = self.poll()
            if status is not None:
                return status
            time.sleep(0.01)
        return None

    def terminate(self) -> None:
        if self.poll() is None:
            try:
                os.kill(self.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            if self.wait(0.5) is None:
                try:
                    os.kill(self.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                self.wait(0.5)
        if not self.closed:
            try:
                os.close(self.fd)
            except OSError:
                pass
            self.closed = True

    def __enter__(self) -> "PtyProcess":
        return self

    def __exit__(self, _type: Any, _value: Any, _traceback: Any) -> None:
        self.terminate()


def _own_process(arguments: Iterable[str]) -> PtyProcess:
    script = str(Path(__file__).resolve())
    return PtyProcess([sys.executable, script, *arguments])


def _percentile(values: list[float], fraction: float) -> float:
    if not values:
        raise ValueError("no values")
    ordered = sorted(values)
    index = max(0, math.ceil(len(ordered) * fraction) - 1)
    return ordered[index]


def _gate(
    name: str,
    value: float,
    unit: str,
    limit: float,
    comparison: str,
    details: Optional[dict[str, Any]] = None,
) -> dict[str, Any]:
    passed = value <= limit if comparison == "<=" else value >= limit
    return {
        "name": name,
        "value": round(value, 3),
        "unit": unit,
        "comparison": comparison,
        "limit": limit,
        "passed": passed,
        "details": details or {},
    }


def _boolean_gate(name: str, passed: bool, details: Optional[dict[str, Any]] = None) -> dict[str, Any]:
    return {
        "name": name,
        "value": bool(passed),
        "unit": "boolean",
        "comparison": "==",
        "limit": True,
        "passed": bool(passed),
        "details": details or {},
    }


def _measure_shell_startup(shell: str, iterations: int, limit_ms: float) -> dict[str, Any]:
    latencies = []
    marker = b"PH7QA_SHELL_READY"
    for _ in range(iterations):
        started = time.perf_counter()
        with PtyProcess(
            [shell, "-l", "-i"],
            env={"TERM": "xterm-256color", "COLORTERM": "truecolor"},
        ) as child:
            child.set_echo(False)
            child.write(b"printf '\\nPH7QA_SHELL_READY\\n'\nexit\n")
            child.read_until(marker, 5.0)
            latencies.append((time.perf_counter() - started) * 1000)
    p95 = _percentile(latencies, 0.95)
    return _gate(
        "host_shell_startup_p95",
        p95,
        "ms",
        limit_ms,
        "<=",
        {"median_ms": round(statistics.median(latencies), 3), "samples": len(latencies), "shell": shell},
    )


def _measure_stream(kind: str, byte_count: int, write_size: int, limit: float) -> dict[str, Any]:
    size_option = "--chunk-size" if kind == "bulk" else "--fragment-size"
    arguments = ["workload", kind, "--bytes", str(byte_count), size_option, str(write_size)]
    started = time.perf_counter()
    tail = bytearray()
    with _own_process(arguments) as child:
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            data = child.read(0.2)
            if data:
                tail.extend(data)
                if len(tail) > 128 * 1024:
                    del tail[: len(tail) - 128 * 1024]
                if RESULT_PREFIX in tail:
                    break
            elif data == b"" and child.poll() is not None:
                break
        else:
            raise TimeoutError(f"{kind} workload exceeded 30 seconds")
        if RESULT_PREFIX not in tail:
            raise RuntimeError(f"{kind} workload exited without a result marker")
    elapsed = max(time.perf_counter() - started, 1e-9)
    throughput = (byte_count / MIB) / elapsed
    return _gate(
        f"host_pty_{kind}_throughput",
        throughput,
        "MiB/s",
        limit,
        ">=",
        {"bytes": byte_count, "write_size": write_size, "wall_seconds": round(elapsed, 4)},
    )


def _check_ansi() -> dict[str, Any]:
    with _own_process(["workload", "ansi", "--frames", "4", "--rows", "12", "--columns", "60"]) as child:
        output = child.read_until(RESULT_PREFIX, 5.0)
    required = (b"\x1b[?1049h", b"\x1b[38;2;", b"\x1b]8;;", b"\x1b[?1049l")
    missing = [sequence.hex() for sequence in required if sequence not in output]
    return _boolean_gate("ansi_control_sequence_integrity", not missing, {"missing_hex": missing})


def _check_unicode() -> dict[str, Any]:
    with _own_process(["workload", "unicode", "--lines", "20"]) as child:
        output = child.read_until(RESULT_PREFIX, 5.0)
    expected = ("終端性能測試", "🧑🏽‍💻", "┌─┬─┐")
    valid_utf8 = True
    try:
        decoded = output.decode("utf-8")
    except UnicodeDecodeError:
        decoded = ""
        valid_utf8 = False
    missing = [value for value in expected if value not in decoded]
    return _boolean_gate("unicode_stream_integrity", valid_utf8 and not missing, {"missing": missing})


def _measure_ctrl_c(iterations: int, limit_ms: float) -> dict[str, Any]:
    latencies = []
    for _ in range(iterations):
        with _own_process(["workload", "interrupt", "--interval", "0"]) as child:
            child.read_until(READY_INTERRUPT, 3.0)
            started = time.perf_counter()
            child.write(b"\x03")
            child.read_until(HANDLED_INTERRUPT, 2.0)
            latencies.append((time.perf_counter() - started) * 1000)
    p95 = _percentile(latencies, 0.95)
    return _gate(
        "host_pty_ctrl_c_p95",
        p95,
        "ms",
        limit_ms,
        "<=",
        {"median_ms": round(statistics.median(latencies), 3), "max_ms": round(max(latencies), 3), "samples": len(latencies)},
    )


def _measure_resize(iterations: int, limit_ms: float) -> dict[str, Any]:
    dimensions = [(24 + index, 90 + index * 3) for index in range(iterations)]
    latencies = []
    with _own_process(["workload", "resize", "--duration", "30"]) as child:
        child.read_until(b"PH7QA_RESIZE ", 3.0)
        for rows, columns in dimensions:
            expected = f'"rows":{rows},"cols":{columns}'.encode()
            started = time.perf_counter()
            child.resize(rows, columns)
            child.read_until(expected, 2.0)
            latencies.append((time.perf_counter() - started) * 1000)
    p95 = _percentile(latencies, 0.95)
    return _gate(
        "host_pty_resize_p95",
        p95,
        "ms",
        limit_ms,
        "<=",
        {"median_ms": round(statistics.median(latencies), 3), "max_ms": round(max(latencies), 3), "samples": len(latencies)},
    )


def _measure_isolation(byte_count: int, iterations: int, limit_ms: float) -> dict[str, Any]:
    flood = _own_process(
        ["workload", "fragmented", "--bytes", str(byte_count), "--fragment-size", "32"]
    )
    echo = _own_process(["workload", "echo", "--duration", "30"])
    flood_done = threading.Event()
    flood_output_bytes = 0
    flood_lock = threading.Lock()

    def drain_flood() -> None:
        nonlocal flood_output_bytes
        while True:
            data = flood.read(0.1)
            if data:
                with flood_lock:
                    flood_output_bytes += len(data)
            elif data == b"":
                break
        flood_done.set()

    drain = threading.Thread(target=drain_flood, name="ph7qa-flood-drain", daemon=True)
    drain.start()
    latencies = []
    leaked = False
    completed_while_flooding = 0
    try:
        echo.read_until(READY_ECHO, 3.0)
        for index in range(iterations):
            token = f"isolation-{index:03d}-{time.monotonic_ns()}"
            expected = f'"token":"{token}"'.encode()
            started = time.perf_counter()
            echo.write(token.encode() + b"\n")
            response = echo.read_until(expected, 2.0)
            latencies.append((time.perf_counter() - started) * 1000)
            leaked = leaked or BULK_PATTERN[:24] in response
            if not flood_done.is_set():
                completed_while_flooding += 1
            time.sleep(0.01)
    finally:
        echo.terminate()
        if flood.poll() is None:
            try:
                os.kill(flood.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        drain.join(timeout=1.0)
        flood.terminate()
    p95 = _percentile(latencies, 0.95)
    latency_gate = _gate(
        "host_pty_cross_session_echo_p95",
        p95,
        "ms",
        limit_ms,
        "<=",
        {
            "median_ms": round(statistics.median(latencies), 3),
            "max_ms": round(max(latencies), 3),
            "samples": len(latencies),
            "samples_while_flooding": completed_while_flooding,
            "flood_output_bytes": flood_output_bytes,
        },
    )
    if completed_while_flooding < max(1, iterations // 2):
        latency_gate["passed"] = False
        latency_gate["details"]["error"] = "flood ended before enough isolation samples"
    leakage_gate = _boolean_gate("host_pty_cross_session_no_leakage", not leaked)
    return {"latency": latency_gate, "leakage": leakage_gate}


def _run_check(name: str, operation: Any) -> list[dict[str, Any]]:
    try:
        result = operation()
        if isinstance(result, dict) and "passed" in result:
            return [result]
        if isinstance(result, dict):
            return list(result.values())
        raise TypeError(f"{name} returned an invalid result")
    except Exception as error:  # Keep the report useful when one subsystem fails.
        return [_boolean_gate(name, False, {"error": f"{type(error).__name__}: {error}"})]


def run_selftest(args: argparse.Namespace) -> int:
    if os.name != "posix":
        raise RuntimeError("the PTY controller requires a POSIX host")
    shell = args.shell or os.environ.get("SHELL") or "/bin/zsh"
    results: list[dict[str, Any]] = []
    checks = (
        ("host_shell_startup", lambda: _measure_shell_startup(shell, args.iterations, args.startup_p95_ms)),
        (
            "bulk_throughput",
            lambda: _measure_stream("bulk", args.bulk_mib * MIB, args.bulk_chunk_size, args.bulk_min_mib_s),
        ),
        (
            "fragmented_throughput",
            lambda: _measure_stream(
                "fragmented",
                args.fragmented_mib * MIB,
                args.fragment_size,
                args.fragmented_min_mib_s,
            ),
        ),
        ("ansi_integrity", _check_ansi),
        ("unicode_integrity", _check_unicode),
        ("ctrl_c", lambda: _measure_ctrl_c(args.iterations, args.ctrl_c_p95_ms)),
        ("resize", lambda: _measure_resize(args.iterations, args.resize_p95_ms)),
        (
            "cross_session_isolation",
            lambda: _measure_isolation(args.isolation_mib * MIB, args.iterations, args.isolation_p95_ms),
        ),
    )
    for name, check in checks:
        results.extend(_run_check(name, check))

    report = {
        "schema_version": 1,
        "generated_at_unix": round(time.time(), 3),
        "host": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "shell": shell,
        },
        "scope": "host PTY floor; run workload subcommands inside pH7Console for end-to-end QA",
        "passed": all(result["passed"] for result in results),
        "results": results,
    }
    if args.output_json:
        Path(args.output_json).expanduser().write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if args.json_stdout:
        print(json.dumps(report, indent=2))
    else:
        print("pH7Console terminal QA — host PTY floor")
        for result in results:
            state = "PASS" if result["passed"] else "FAIL"
            if result["unit"] == "boolean":
                measurement = str(result["value"])
            else:
                measurement = f"{result['value']} {result['unit']} ({result['comparison']} {result['limit']})"
            print(f"[{state}] {result['name']}: {measurement}")
            if not result["passed"] and result["details"]:
                print(f"       {json.dumps(result['details'], sort_keys=True)}")
        print("Overall:", "PASS" if report["passed"] else "FAIL")
        print("See docs/terminal-performance-qa.md for end-to-end app gates.")
    return 0 if report["passed"] else 2


def _process_table() -> dict[int, tuple[int, int]]:
    completed = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,rss="],
        check=True,
        capture_output=True,
        text=True,
        env={**os.environ, "LC_ALL": "C"},
    )
    table: dict[int, tuple[int, int]] = {}
    for line in completed.stdout.splitlines():
        fields = line.split()
        if len(fields) == 3:
            pid, parent, rss = map(int, fields)
            table[pid] = (parent, rss)
    return table


def _descendants(root: int, table: dict[int, tuple[int, int]]) -> set[int]:
    selected = {root}
    changed = True
    while changed:
        changed = False
        for pid, (parent, _rss) in table.items():
            if parent in selected and pid not in selected:
                selected.add(pid)
                changed = True
    return selected


def monitor_memory(args: argparse.Namespace) -> int:
    samples = []
    deadline = time.monotonic() + args.seconds
    while time.monotonic() < deadline or not samples:
        table = _process_table()
        if args.pid not in table:
            if not samples:
                raise RuntimeError(f"PID {args.pid} does not exist or is not visible")
            break
        pids = _descendants(args.pid, table) if args.include_children else {args.pid}
        rss_mib = sum(table[pid][1] for pid in pids if pid in table) / 1024
        sample = {"elapsed_seconds": round(args.seconds - max(0.0, deadline - time.monotonic()), 3), "rss_mib": round(rss_mib, 3), "processes": len(pids)}
        samples.append(sample)
        if args.live:
            print(json.dumps(sample, separators=(",", ":")), flush=True)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        time.sleep(min(args.interval, remaining))
    values = [sample["rss_mib"] for sample in samples]
    maximum = max(values)
    passed = args.max_mib is None or maximum <= args.max_mib
    report = {
        "kind": "memory",
        "pid": args.pid,
        "include_children": args.include_children,
        "samples": len(samples),
        "median_rss_mib": round(statistics.median(values), 3),
        "max_rss_mib": round(maximum, 3),
        "limit_mib": args.max_mib,
        "passed": passed,
    }
    print("PH7QA_MEMORY " + json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0 if passed else 2


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Generate terminal stress workloads and measure the host PTY performance floor.",
        epilog="End-to-end procedures and product acceptance gates: docs/terminal-performance-qa.md",
    )
    parser.add_argument("--version", action="version", version="terminal_qa.py 1.0")
    commands = parser.add_subparsers(dest="command", required=True)

    workload = commands.add_parser("workload", help="emit a workload inside the terminal under test")
    workloads = workload.add_subparsers(dest="workload", required=True)

    bulk = workloads.add_parser("bulk", help="emit deterministic high-throughput printable output")
    bulk.add_argument("--bytes", type=_positive_int, default=100 * MIB)
    bulk.add_argument("--chunk-size", type=_positive_int, default=64 * 1024)
    bulk.set_defaults(func=workload_bulk)

    fragmented = workloads.add_parser("fragmented", help="emit deterministic output in tiny writes")
    fragmented.add_argument("--bytes", type=_positive_int, default=16 * MIB)
    fragmented.add_argument("--fragment-size", type=_positive_int, default=64)
    fragmented.add_argument("--delay-us", type=_non_negative_float, default=0.0)
    fragmented.set_defaults(func=workload_fragmented)

    ansi = workloads.add_parser("ansi", help="animate ANSI, truecolor, cursor, title, and OSC 8 sequences")
    ansi.add_argument("--frames", type=_positive_int, default=600)
    ansi.add_argument("--rows", type=_positive_int, default=30)
    ansi.add_argument("--columns", type=_positive_int, default=100)
    ansi.add_argument("--delay-ms", type=_non_negative_float, default=0.0)
    ansi.set_defaults(func=workload_ansi)

    unicode_workload = workloads.add_parser("unicode", help="emit grapheme, RTL, CJK, emoji, and box-drawing cases")
    unicode_workload.add_argument("--lines", type=_positive_int, default=10_000)
    unicode_workload.set_defaults(func=workload_unicode)

    interrupt = workloads.add_parser("interrupt", help="wait for Ctrl-C and emit a timestamped acknowledgement")
    interrupt.add_argument("--interval", type=_non_negative_float, default=0.05, help="heartbeat interval in seconds; zero is silent")
    interrupt.set_defaults(func=workload_interrupt)

    resize = workloads.add_parser("resize", help="report initial size and every SIGWINCH with timestamps")
    resize.add_argument("--duration", type=_non_negative_float, default=30.0)
    resize.set_defaults(func=workload_resize)

    echo = workloads.add_parser("echo", help="timestamp each input line for cross-session responsiveness checks")
    echo.add_argument("--duration", type=_non_negative_float, default=60.0)
    echo.set_defaults(func=workload_echo)

    selftest = commands.add_parser("selftest", help="run automated, non-visual host PTY acceptance checks")
    selftest.add_argument("--shell", help="login shell to benchmark; defaults to SHELL")
    selftest.add_argument("--iterations", type=_positive_int, default=7)
    selftest.add_argument("--bulk-mib", type=_positive_int, default=32)
    selftest.add_argument("--bulk-chunk-size", type=_positive_int, default=64 * 1024)
    selftest.add_argument("--fragmented-mib", type=_positive_int, default=8)
    selftest.add_argument("--fragment-size", type=_positive_int, default=64)
    selftest.add_argument("--isolation-mib", type=_positive_int, default=16)
    selftest.add_argument("--startup-p95-ms", type=_non_negative_float, default=700.0)
    # Python's byte-by-byte PTY controller is intentionally conservative. This
    # is a machine-health floor, not the end-to-end app release gate documented
    # in docs/terminal-performance-qa.md.
    selftest.add_argument("--bulk-min-mib-s", type=_non_negative_float, default=3.0)
    selftest.add_argument("--fragmented-min-mib-s", type=_non_negative_float, default=1.0)
    selftest.add_argument("--ctrl-c-p95-ms", type=_non_negative_float, default=100.0)
    selftest.add_argument("--resize-p95-ms", type=_non_negative_float, default=100.0)
    selftest.add_argument("--isolation-p95-ms", type=_non_negative_float, default=75.0)
    selftest.add_argument("--json-stdout", action="store_true")
    selftest.add_argument("--output-json", help="optional path for a machine-readable report")
    selftest.set_defaults(func=run_selftest)

    memory = commands.add_parser("memory", help="sample RSS for an application PID without modifying it")
    memory.add_argument("--pid", type=_positive_int, required=True)
    memory.add_argument("--seconds", type=_non_negative_float, default=30.0)
    memory.add_argument("--interval", type=_positive_float, default=0.5)
    memory.add_argument("--include-children", action="store_true", help="sum the full descendant process tree")
    memory.add_argument("--max-mib", type=_non_negative_float, help="return status 2 if peak RSS exceeds this gate")
    memory.add_argument("--live", action="store_true")
    memory.set_defaults(func=monitor_memory)
    return parser


def main(argv: Optional[list[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except BrokenPipeError:
        return 0
    except KeyboardInterrupt:
        return 130
    except Exception as error:
        print(f"terminal_qa.py: {type(error).__name__}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
