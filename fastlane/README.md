fastlane documentation
----

# Installation

Make sure you have the latest version of the Xcode command line tools installed:

```sh
xcode-select --install
```

For _fastlane_ installation instructions, see [Installing _fastlane_](https://docs.fastlane.tools/#installing-fastlane)

# Available Actions

## Mac

### mac ensure_appstore_record

```sh
[bundle exec] fastlane mac ensure_appstore_record
```

Create the pH7Console bundle ID and App Store Connect record when missing

### mac upload_build_via_api

```sh
[bundle exec] fastlane mac upload_build_via_api
```

Upload a signed Mac App Store package using Apple's Build Upload API

### mac wait_for_build_upload_processing

```sh
[bundle exec] fastlane mac wait_for_build_upload_processing
```

Wait for Apple's newest pH7Console build upload to finish processing

### mac verify_uploaded_build

```sh
[bundle exec] fastlane mac verify_uploaded_build
```

Verify that the expected pH7Console 1.0.0 build reached App Store Connect

### mac select_uploaded_build

```sh
[bundle exec] fastlane mac select_uploaded_build
```

Select the validated App Store build for version 1.0.0

### mac audit_submission_readiness

```sh
[bundle exec] fastlane mac audit_submission_readiness
```

Report submission-readiness fields without exposing contact values

### mac upload_content_rights

```sh
[bundle exec] fastlane mac upload_content_rights
```

Set and verify Content Rights for the bundled Qwen and llama.cpp components

### mac upload_store_listing

```sh
[bundle exec] fastlane mac upload_store_listing
```

Upload the English Mac App Store listing and screenshots without submitting for review

### mac upload_store_screenshots

```sh
[bundle exec] fastlane mac upload_store_screenshots
```

Upload Mac App Store screenshots independently of review contact metadata

### mac upload_store_age_rating

```sh
[bundle exec] fastlane mac upload_store_age_rating
```

Upload the age-rating declaration without requiring review-contact metadata

### mac upload_review_information

```sh
[bundle exec] fastlane mac upload_review_information
```

Upload App Review contact and notes after a phone number is supplied

### mac upload_review_notes

```sh
[bundle exec] fastlane mac upload_review_notes
```

Upload App Review notes while preserving the existing reviewer contact

### mac upload_privacy_policy_url

```sh
[bundle exec] fastlane mac upload_privacy_policy_url
```

Upload the public privacy-policy URL after it is published

### mac set_free_pricing

```sh
[bundle exec] fastlane mac set_free_pricing
```

Set pH7Console to a permanent Free price in every App Store territory

### mac submit_for_review

```sh
[bundle exec] fastlane mac submit_for_review
```

Verify all gates and submit the selected macOS build for App Review

### mac verify_store_listing

```sh
[bundle exec] fastlane mac verify_store_listing
```

Verify the accepted App Store text and Mac screenshot count

### mac deduplicate_store_screenshots

```sh
[bundle exec] fastlane mac deduplicate_store_screenshots
```

Remove App Store screenshot duplicates caused by asynchronous upload retries

### mac prepare_signing

```sh
[bundle exec] fastlane mac prepare_signing
```

Create and install Mac App Store certificates and download the provisioning profile

----

This README.md is auto-generated and will be re-generated every time [_fastlane_](https://fastlane.tools) is run.

More information about _fastlane_ can be found on [fastlane.tools](https://fastlane.tools).

The documentation of _fastlane_ can be found on [docs.fastlane.tools](https://docs.fastlane.tools).
