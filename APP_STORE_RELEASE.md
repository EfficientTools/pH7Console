# Mac App Store release guide

pH7Console uses a separate App Store configuration so direct-download builds can remain independent from the App Sandbox configuration required by Apple.

## 1. Apple account setup

1. Enrol in the Apple Developer Program.
2. Register the explicit bundle ID `com.efficienttools.ph7console`.
3. Create an **Apple Distribution** certificate.
4. Create a **Mac Installer Distribution** certificate.
5. Create a **Mac App Store Connect** provisioning profile for the bundle ID and download it.
6. Create the macOS app record in App Store Connect with the same bundle ID.

When using an App Store Connect API key with certificate access, the repository can create the Developer bundle ID, certificates, and provisioning profile automatically. Apple currently requires the App Store Connect app record itself to be created in the web interface:

```bash
export APPLE_TEAM_ID="YOURTEAMID"
export APPLE_API_KEY_ID="KEY_ID"
export APPLE_API_ISSUER="ISSUER_UUID"
export APPLE_API_KEY_PATH="/absolute/path/to/AuthKey_KEY_ID.p8"
npm run appstore:prepare-signing
```

This installs the generated certificates in the login keychain and writes the provisioning profile to `.appstore/signing/pH7Console.mobileprovision`. The ignored `.appstore` directory is kept outside the frontend build output so Vite cannot erase the profile. Temporary private-key export files are removed after keychain import.

After upload, `npm run appstore:verify-build` confirms that version 1.0.0 build 3 reached App Store Connect and reports Apple's processing state.

The store listing is managed from `fastlane/metadata`, with Apple-compliant 2880×1800 screenshots in `fastlane/screenshots/en-US`:

```bash
npm run appstore:upload-listing
npm run appstore:upload-screenshots
npm run appstore:deduplicate-screenshots
npm run appstore:verify-listing
```

`appstore:upload-listing` requires every mandatory review-contact field, including an international-format phone number. Screenshot upload is separate so artwork can still be published while review contact details are being finalized. The deduplication lane is safe to run repeatedly and handles delayed App Store Connect processing that can make an upload retry appear twice.

## 2. Local release environment

Install the two certificates in the login keychain and export these values in the release shell:

```bash
export APPLE_TEAM_ID="YOURTEAMID"
export APPLE_SIGNING_IDENTITY="Apple Distribution: Your Name (YOURTEAMID)"
export APPLE_INSTALLER_SIGNING_IDENTITY="3rd Party Mac Developer Installer: Your Name (YOURTEAMID)"
export APPLE_PROVISIONING_PROFILE="/absolute/path/to/pH7Console.provisionprofile"
export APP_BUILD_NUMBER="4"
```

`APP_BUILD_NUMBER` is mandatory and must be a positive integer greater than every previously uploaded build. Version 1.0.0 build 4 is the current release candidate. The build script fails closed if the value is unset, malformed, or less than 4. App Store Connect build numbers are immutable. The certificate label can vary by Apple account age. Use the exact label shown by `security find-identity -v -p codesigning` and Keychain Access.

## 3. Build and validate

```bash
npm run tauri:build:appstore
```

The script verifies the signing certificates and provisioning profile, creates a signed universal app, verifies its signature and entitlements, then creates `dist/app-store/pH7Console.pkg` signed for App Store installation. It requires the pinned Qwen GGUF SHA-256 (`cc324af070c2ecbfd324a30884d2f951a7ff756aba85cb811a6ec436933bb046`) before and after bundling, checks exact arm64/x86_64 slices and system-only linkage, gives the llama helper the stable code identifier `com.efficienttools.ph7console.llama-server`, and validates the final versions and resources. Cargo runs offline and the dependency lockfiles must remain unchanged. Generated configuration, entitlements, and the embedded provisioning profile are removed from the source tree when the build ends.

The pipeline refuses to overwrite an existing `.app` or `.pkg`, and it rejects stale generated signing inputs, private-key-like files inside the repository, unexpected models/helpers/Mach-O files, symlinked resources, and quarantine/resource-fork contamination. Move or remove a stale output deliberately before retrying; the release script does not clean it automatically.

The app bundle includes `PrivacyInfo.xcprivacy` in `Contents/Resources`, declaring that the app performs no tracking and collects no data. Optional voice input uses Apple's on-device recognizer only, does not persist audio, and produces an editable command-planning draft that is never executed automatically. The build verifies the microphone entitlement and both macOS privacy usage descriptions. Keep this manifest, `PRIVACY.md`, and the App Store Connect privacy answers synchronized if data handling changes.

Test the sandboxed `.app` on both Apple Silicon and Intel hardware, or through representative TestFlight coverage. In particular verify workspace selection, shell execution, networking, file creation, and behavior after relaunch.

## 4. Upload

Create an App Store Connect API key, store its private `AuthKey_<KEY_ID>.p8` file in one of Apple's supported private-key locations, then set:

```bash
export APPLE_API_KEY_ID="KEY_ID"
export APPLE_API_ISSUER="ISSUER_UUID"
export APPLE_API_KEY_PATH="/absolute/path/to/AuthKey_KEY_ID.p8"
npm run tauri:upload:appstore
```

`APPLE_API_KEY_PATH` is required. The upload script uses Apple's Build Upload API, transfers the package in Apple-reserved parts, marks the file complete, and waits for delivery processing to finish. It does not print credentials or signed upload URLs.

## 5. App Store Connect submission

- Publish `PRIVACY.md` at a stable public HTTPS URL, then upload it with `APP_PRIVACY_POLICY_URL=https://... fastlane mac upload_privacy_policy_url`.
- Complete App Privacy in the signed-in App Store Connect web interface. The intended declaration is **No, we do not collect data from this app**: pH7Console has no telemetry or cloud service, command output is not persisted by default, adaptive state is rebuilt locally from encrypted redacted history, and local state never leaves the Mac. Apple requires an authorized user to confirm the declaration and click **Publish**; this legal confirmation is not exposed by the supported API.
- Complete export compliance consistently with `ITSAppUsesNonExemptEncryption=false`. Reassess this answer if cryptography is added to the app itself.
- Verify the permanent Free price and territories with `fastlane mac set_free_pricing`.
- Upload the prepared App Review contact and notes with `APP_REVIEW_PHONE=+... fastlane mac upload_review_information`.
- When reviewer contact information already exists, update notes without replacing it using `fastlane mac upload_review_notes`.
- The `com.apple.security.network.server` entitlement is required by the signed, sandbox-inheriting `llama-server` helper. It listens only on authenticated `127.0.0.1` with an ephemeral port for on-device inference. Keep this exact justification and the reviewer verification steps in App Review Notes; removing the entitlement breaks local AI because a directly launched helper inherits the parent app's sandbox capabilities.
- After voice-input QA, re-run `fastlane mac audit_submission_readiness`, confirm build 4 is selected, and submit version 1.0.0 for review.

## Version 1.0.0 status

- App Store Connect record: `6791753004`
- Bundle ID: `com.efficienttools.ph7console`
- Build 4: signed universal package uploaded through Apple's Build Upload API, processed as **VALID**, and selected for version 1.0.0
- English metadata: accepted by App Store Connect
- Content Rights: **Uses Third-Party Content**, retained and verified through the API for bundled Qwen and llama.cpp licenses
- Privacy Policy URL: published and retained by App Store Connect
- Pricing: permanent **Free** price retained and verified through the current pricing API
- Age rating: accepted by App Store Connect
- Mac screenshots: five unique accepted images in final display order, verified through the API
- Release mode: automatic release after approval
- Remaining before review submission: public privacy-policy URL, international-format review phone number, published **Data Not Collected** declaration, and pricing/territory selection

## Sandbox product constraint

The Mac App Store build cannot behave like an unrestricted Terminal.app replacement. Apple's App Sandbox limits child processes and filesystem access. pH7Console therefore uses the system folder picker to grant access to a user-selected workspace. Keep a separately signed and notarized direct-download edition if unrestricted terminal access remains a product requirement.
