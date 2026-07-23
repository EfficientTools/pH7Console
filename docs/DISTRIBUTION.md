# macOS distribution model

pH7Console needs two separately built and qualified macOS editions. A single artifact cannot both comply with the Mac App Store App Sandbox and behave like a general terminal for the user's whole account.

In this document, “direct edition” means an app that is not in the App Sandbox. It does **not** mean root access or a bypass of macOS security. The direct edition still runs as the signed-in user and remains subject to POSIX permissions, Transparency, Consent, and Control (TCC), System Integrity Protection, Gatekeeper, and any managed-device policy.

## Edition boundaries

| | Mac App Store workspace edition | Direct notarized edition |
|---|---|---|
| Distribution | App Store Connect and the Mac App Store | Project-controlled HTTPS download |
| Sandbox | Required | Not enabled |
| Intended use | A terminal for a folder the user explicitly selects | A general terminal for resources the signed-in user can access |
| File access | User-selected read/write/executable entitlement, plus the limits applied to sandboxed child processes | Normal user permissions and macOS privacy prompts |
| Signing | Apple Distribution app signature, provisioning profile, and Mac Installer Distribution package signature | Developer ID Application signature with hardened runtime, followed by Apple notarization and stapling |
| Package today | `scripts/build-appstore.sh` creates `dist/app-store/pH7Console.pkg` | No release-qualified direct package or notarization script exists yet |
| Updates | Mac App Store | A separately secured update channel or signed replacement download is required |
| “Default terminal” scope | Launcher/deep-link integrations only; not a system-wide unrestricted terminal | Can be selected by tools that accept a terminal application, but macOS still has no universal default-terminal preference |

The Mac App Store identifier is `com.efficienttools.ph7console`. The base Tauri configuration currently gives a direct build the same identifier. Before a direct release, decide whether the two editions must coexist. If they must, give the direct edition a separate bundle identifier and update channel; otherwise installing one edition may replace or conflict with the other.

## Mac App Store workspace edition

The App Store build merges `src-tauri/tauri.appstore.conf.json` into the base Tauri configuration. Its generated entitlements are:

- App Sandbox;
- user-selected file read/write access;
- execution of files selected by the user;
- network client and server access; and
- the application and Team identifiers required by the provisioning profile.

The network entitlements allow commands the user runs, and an optional local loopback inference service, to use networking. They do not authorize telemetry or a cloud AI service. Their purpose must be explained in App Review notes.

The store edition must be described as a workspace console, not as an unrestricted replacement for Terminal.app. Selecting a folder does not remove the App Sandbox, and some programs or child-process workflows can still fail when they expect access outside the granted scope. Access after relaunch must be tested; it must not be promised unless the release preserves and successfully restores the appropriate macOS authorization.

The existing App Store script builds a universal arm64/x86_64 app, checks its provisioning profile, architecture, signature, privacy manifest, and sandbox entitlements, and signs a `.pkg`. It does not make the product review-ready by itself.

Current App Store Connect state for version 1.0.0:

- App record `6791753004` exists.
- Build 2 was uploaded and processed as `VALID`.
- English metadata and five macOS screenshots were accepted by App Store Connect.
- A public privacy-policy URL, review phone number, App Privacy questionnaire, age rating, pricing/availability, and build selection are still required.
- The current working-tree terminal, history, deep-link, and optional LLM changes are not proven to be part of build 2. The next upload must use a build number greater than 2 and repeat all validation.

## Direct notarized edition

The direct edition is the appropriate channel for a full user-account terminal. It may start a login shell with the same file access the user has, subject to normal macOS controls. It must not request root privileges for ordinary terminal operation.

The repository can produce an unsigned or ad-hoc Tauri application from the base configuration, but it does not yet contain a complete Developer ID release pipeline. A direct release needs all of the following before it is offered to users:

- a dedicated release configuration and an intentional bundle-identifier decision;
- Developer ID signing with hardened runtime;
- inside-out signing and verification of every nested executable and dynamic library;
- a signed DMG or installer package, as appropriate;
- notarization, stapling, and a Gatekeeper assessment on a clean Mac;
- an upgrade and rollback policy; and
- installation and removal instructions for the optional `ph7` command-line launcher.

Removing the App Sandbox increases capability and responsibility. The direct build must retain command-plan confirmation, deep-link validation, local-history redaction, and least-privilege behavior even though the operating system no longer confines it to a selected workspace.

## Default-terminal integration: what macOS permits

macOS does not expose one system preference that replaces every use of Terminal.app. Terminal selection is normally per application, per URL/file handler, or implemented by a launcher. pH7Console therefore must not claim that installing it changes a universal “default terminal.”

The following integration exists in the source tree:

- The app registers the `ph7console` URL scheme through the Tauri deep-link plugin.
- `ph7console://new` opens a new session.
- `cwd64` supplies a working directory as unpadded base64url text.
- `command64` supplies command text as unpadded base64url text. It is inserted into the PTY without an Enter key, so the user must review and execute it explicitly.
- `scripts/ph7` generates this URL safely. `ph7 [DIRECTORY] [--command TEXT]` defaults to the caller's current directory and resolves it before launching the app.

From a repository checkout, the current launcher can be used as:

```sh
./scripts/ph7 ~/Code
./scripts/ph7 ~/Code --command 'git status'
```

The script is listed as an application resource, but it is not installed into `PATH`. The Mac App Store app must not silently write a command into `/usr/local/bin`. A direct installer may offer a clearly disclosed, user-approved CLI installation; that installer and its uninstall path are still pending.

The raw deep link is intended for integrations that cannot invoke the script:

```text
ph7console://new?cwd64=<unpadded-base64url>&command64=<unpadded-base64url>
```

External input remains untrusted even though a command is not auto-executed. Path validation, length limits, command prefill-only behavior, and explicit execution must remain security invariants.

The repository does not currently configure `.command` or `.tool` file associations, a Finder extension, a macOS Service/Quick Action, or an installer that registers pH7Console with third-party IDEs. Those are possible follow-up integrations, not current capabilities. File associations would still be handlers for those file types, not a global default terminal.

## Local intelligence and LLM packaging

The deterministic command-matching and learning engine is the always-available local fallback. Genuine LLM inference is optional. The current runtime can:

- use an existing GGUF model at `PH7_LOCAL_MODEL_PATH` or the app-data model location;
- launch a compatible `llama-server` found at one of the expected locations inside the application bundle; or
- connect to a compatible service on `127.0.0.1:8080` after verifying its health and model-list responses.

Managed inference binds to a loopback address, chooses a local port, uses a per-launch API key, disables the server web UI/agent features, and streams responses. Non-loopback endpoints are rejected. If no verified runtime is present, the UI reports that local command intelligence is available rather than claiming a local LLM is running.

This is not yet a published LLM product. The Tauri configuration now declares a universal `llama-server` helper and a pinned Qwen GGUF resource. The App Store build pipeline requires the model SHA-256 `cc324af070c2ecbfd324a30884d2f951a7ff756aba85cb811a6ec436933bb046`, checks it again inside the bundle, and signs the helper as `com.efficienttools.ph7console.llama-server`. No build containing those assets has yet completed the release gates or App Review. The runtime does not download executable code or models, and there is not yet a signed model-manifest/update system.

Before either edition advertises a bundled local LLM:

1. Pin and audit a llama.cpp revision, its build flags, dependencies, and licenses.
2. Produce arm64 and x86_64 helpers, or a verified universal helper, and test Metal and CPU fallback behavior on supported Macs.
3. Place the helper in a declared bundle location and sign every nested Mach-O dependency before signing the main app.
4. For the App Store build, sign the helper with the distribution identity and sandbox-compatible entitlements, include it before package signing, and verify it inside the final `.app`. A sandboxed child remains sandboxed.
5. For the direct build, sign the helper with Developer ID and hardened runtime before signing and notarizing the outer app.
6. Treat GGUF files as data, but distribute them with a signed manifest containing a cryptographic digest, source, license, size, architecture/hardware guidance, and model version.
7. Add explicit model install/remove/update controls and disk-space/memory checks; expand cancellation, crash-recovery, and deterministic-fallback qualification.
8. Reassess App Store package size, review requirements, export compliance, privacy wording, and third-party notices.

A separately run loopback server is controlled by the user, not by pH7Console. Prompts sent to it stay on the local network stack, but that server is a separate trusted process with its own configuration and data-handling responsibility.

## Privacy invariants

Both editions must have the same no-telemetry default:

- commands the user runs may themselves contact the network; and
- release logs and diagnostics must not print commands, output, credentials, environment variables, or App Store credentials.

The current history implementation stores a SQLCipher-encrypted SQLite database in the app-data directory. It persists bounded, heuristically redacted command text plus working directory, shell, timing, and exit status. The database, FTS5 index, and WAL pages are encrypted with a random 256-bit key held in a device-only, non-synchronizing macOS data-protection Keychain item. Output excerpts are disabled by default. The default retention policy caps non-running history at 25,000 records and 180 days while preserving starred records. Database work is kept off the PTY I/O path and FTS5 supports local search. Adaptive workflow state is memory-only and is rebuilt from up to 500 encrypted history records on launch; clearing history also clears that in-memory state.

Redaction reduces risk; it cannot guarantee that every secret embedded in a command is detected. The History window discloses encrypted versus memory-only mode and provides a durable **Clear all** action. A Keychain, wrong-key, corruption, or migration error fails closed to the bounded in-memory cache and never falls back to plaintext persistence. The App Store “Data Not Collected” answer can remain accurate only if this data stays on the device and is neither collected by the developer nor transmitted, but the public policy still needs to describe the on-device retention precisely.

SQLCipher is an export-compliance-relevant cryptographic feature. `Info.plist` currently declares `ITSAppUsesNonExemptEncryption` as `false`; that value is correct only if the developer's current Apple export-classification answers establish that the bundled data-at-rest encryption is exempt. Revalidate the App Store Connect export-compliance questionnaire for this build and complete any classification or reporting Apple requires before submission. The build pipeline must not infer that classification from the implementation alone.

The bounded PTY replay buffer and xterm scrollback hold output in memory for terminal operation. Persistent output storage must remain opt-in, bounded, redacted, and documented if it is enabled in a future release.

## Current implementation versus release work

| Area | Present in the current source | Still pending before a production claim |
|---|---|---|
| Terminal core | Persistent native PTY per tab; independent input/output; raw text and binary input; real resize, interrupt, exit, bounded snapshot replay, and same-workspace shell restart | Broad shell/TUI/SSH/tmux compatibility matrix, soak testing, process-crash recovery, and measured latency/throughput baselines |
| Renderer | xterm.js with fit, search, links, active-tab WebGL promotion, canvas fallback, true-color configuration, and bounded scrollback | Accessibility and rendering verification across supported macOS versions and Intel/Apple Silicon hardware |
| Shell integration | Private zsh, bash, and fish wrappers with OSC 7/133 lifecycle data; user startup files are not edited | Compatibility testing for complex user startup files; semantic command capture for other shells |
| Sessions | Independent PTYs and per-session working-directory metadata | Restorable sessions across app crashes/restarts and explicit lifecycle UX where required |
| History | SQLCipher-encrypted SQLite WAL/FTS5 store, Keychain key, bounded background writes, retention, heuristic secret redaction, visible memory-only fallback, and durable clear action; output excerpts off by default | Optional user-configurable retention/disable controls and richer migration/repair UX |
| Command safety | Natural-language output is presented as a risk-labelled plan; the AI panel inserts commands without executing them | Adversarial prompt/deep-link tests and a documented safety policy for every external integration |
| Launcher | `ph7console://new` handling and the `scripts/ph7` launcher | PATH installer, uninstall flow, third-party terminal selection instructions, and optional file/Service integrations |
| Local reasoning | Deterministic local fallback, cancellable generation, encrypted-history-backed workflow adaptation, verified loopback/managed llama.cpp provider code, universal helper input, pinned Qwen GGUF, and fail-closed App Store bundle checks | A successfully signed and qualified build, model lifecycle UI, hardware qualification, signed model manifests, and App Review |
| App Store package | Universal sandbox packaging and upload automation; build 2 is valid in App Store Connect | Build greater than 2 containing the current source, TestFlight qualification, remaining metadata/questionnaires, and App Review approval |
| Direct package | Base non-sandbox Tauri build | Developer ID release automation, nested-code signing, notarization/stapling, installer/update channel, and clean-Mac verification |
| Competitive performance | No validated claim | Reproducible benchmarks against declared hardware and workloads; no claim that pH7Console outperforms Warp or another terminal until results support it |

## Release gates

No edition should be published from a dirty working tree or solely because it builds locally. Use a versioned source revision and archive the exact configuration, dependency locks, signatures, and verification output for each artifact.

### Gates shared by both editions

- `npm run type-check`, lint, frontend unit tests, Rust tests, and production frontend/Tauri builds pass from a clean checkout.
- Rust formatting and Clippy checks pass for the release toolchain.
- End-to-end tests cover session creation/closure, typing, Unicode and binary input, resize, Ctrl-C, large output, reconnect/snapshot replay, multiple busy sessions, and deep-link prefill without execution.
- Manual compatibility covers zsh, bash, fish, full-screen TUIs, SSH, tmux, copy/paste, search, links, alternate screen, and suspend/resume on supported macOS versions.
- Startup-to-prompt, key-to-echo, resize, sustained-output throughput, peak memory, idle CPU, and cross-session interference are measured on at least a baseline Intel Mac and Apple Silicon Mac. Publish no comparative performance statement until the benchmark procedure and results are reproducible.
- Security review covers shell/path quoting, untrusted OSC data, untrusted deep links, URL-size limits, local-history redaction, loopback-provider verification, and absence of command/output/environment logging.
- Local data migration, retention, deletion, corruption fallback, and uninstall behavior are tested.
- Privacy policy, privacy manifest, in-app wording, model licenses, and third-party notices match the artifact exactly.
- The app remains fully usable through the deterministic engine when the optional model is absent, incompatible, slow, cancelled, or crashes.

### Additional Mac App Store gates

- Set `APP_BUILD_NUMBER` to a value greater than 2 and run `npm run tauri:build:appstore`.
- Verify arm64 and x86_64 slices, the embedded provisioning profile, App Sandbox and Team/application identifiers, nested code signatures, and `PrivacyInfo.xcprivacy` in the final app.
- Install the generated package and run representative TestFlight testing, including selected-folder access and relaunch behavior.
- Confirm that every helper is bundled before signing and works within the same sandbox. Do not download or replace executable code after review.
- Complete the public privacy URL, review contact, App Privacy, age rating, pricing/availability, export-compliance answer, build selection, and review notes.
- Validate, upload, wait for processing, select the processed build, and submit it for App Review. A successful upload is not publication approval.

### Additional direct-edition gates

- Build from a dedicated direct-release configuration and record the chosen bundle identifier.
- Sign nested helpers/libraries first, then the main app, using Developer ID Application and hardened runtime; verify the sealed bundle with strict signature checks.
- Notarize the final distributed artifact, staple the ticket, and run Gatekeeper assessment after downloading it on a clean Mac.
- If a package installs `ph7` into a system path, disclose that payload, request authorization through the installer, prevent command injection, and provide an uninstall procedure.
- Secure the update channel with signed manifests and rollback protection, or explicitly ship without automatic updates.
- Test access behavior under TCC, managed-device restrictions, multiple user accounts, and standard non-admin accounts. Do not describe non-sandboxed access as root or as bypassing macOS controls.
