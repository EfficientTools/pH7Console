# pH7Console Privacy Policy

Effective date: July 19, 2026

pH7Console is designed to keep terminal activity on the Mac where it is created. The app does not require an account and does not collect, transmit, sell, or share personal data. Its bundled command model runs locally through a loopback-only helper; prompts and terminal context are not sent to a remote AI service.

## Data processed on your Mac

pH7Console processes shell commands, command output, selected workspace paths, app settings, and adaptive command patterns only to provide its terminal, suggestion, history, and learning features. This processing happens on the device.

The app retains settings and encrypted command history inside its sandboxed app container. Searchable history can retain command text, working directory, shell, timestamps, duration, and exit status. Likely credentials are heuristically redacted before persistence, history is automatically limited by age and count, and command output is not persisted by default. Redaction is a safety layer, not a guarantee that arbitrary command text contains no sensitive information. Workflow adaptation itself remains in memory and is not written to a separate learning file; on launch, it is rebuilt locally from up to 500 records in encrypted history. Clearing history also clears this adaptive memory.

Optional voice input starts only when you press **Voice** beside the command-planning field. It requires Apple's on-device speech recognition and has no cloud fallback. Microphone audio is held only long enough for live transcription, is never written to disk, and is not collected or sent by pH7Console. The resulting text remains an editable draft; it is never inserted into or executed by the shell automatically.

Local history is stored in a permissions-restricted SQLCipher database inside the app container. SQLCipher encrypts the database, full-text search index, and write-ahead log. Its random 256-bit key is stored in the macOS data-protection Keychain as a device-only, non-synchronizing item. If that key is unavailable or the database cannot be authenticated, pH7Console fails closed and uses memory-only history rather than writing plaintext; the History window shows which mode is active.

The first encrypted release migrates a valid history database created by an older release, verifies the encrypted copy, atomically replaces the live database, and removes the live plaintext files. Because APFS, backups, or snapshots may retain deleted blocks outside the app's control, migration cannot guarantee secure erasure of historical plaintext copies. You can remove current command records and adaptive memory with **Clear all** in the History window. Uninstalling the app removes its container; macOS may retain the now-inert Keychain item, which is rotated if the database no longer exists on a later installation.

## Files and folders

The Mac App Store edition is sandboxed. It can access a workspace only after you select that folder with the macOS folder picker. Access remains subject to macOS permissions and can be revoked through system settings or by removing the app.

## Network access

pH7Console has no telemetry, advertising SDK, third-party analytics, cloud account, or remote AI service. The app does not initiate network transfers of terminal activity.

Commands that you explicitly choose to run may access the network—for example, `git pull`, package installation, or a command that calls a web service. Those transfers are performed by the selected command and its destination, not by pH7Console analytics or AI features.

## Children’s privacy

pH7Console is a general-purpose developer tool and is not directed to children. Because the app does not collect personal data, it does not knowingly collect data from children.

## Changes

If this policy changes, the effective date above will be updated. A future version that introduces data collection will disclose that change before release and update its App Store privacy information.

## Contact

Questions about this policy can be sent to [pierre@ph7.me](mailto:pierre@ph7.me).
