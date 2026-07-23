use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

const EVENT_NAME: &str = "voice-input";
const MAX_NATIVE_EVENT_BYTES: usize = 32 * 1024;
const MAX_TRANSCRIPT_CHARS: usize = 4_000;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VoiceEvent {
    pub kind: String,
    pub transcript: Option<String>,
    pub is_final: bool,
    pub available: bool,
    pub on_device_available: bool,
    pub microphone_authorization: String,
    pub speech_authorization: String,
    pub message: String,
}

impl Default for VoiceEvent {
    fn default() -> Self {
        Self {
            kind: "status".to_string(),
            transcript: None,
            is_final: false,
            available: false,
            on_device_available: false,
            microphone_authorization: "unknown".to_string(),
            speech_authorization: "unknown".to_string(),
            message: "Voice input is unavailable on this platform.".to_string(),
        }
    }
}

pub fn initialize(app_handle: AppHandle) {
    let _ = APP_HANDLE.set(app_handle);
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn sanitize_transcript(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            (!character.is_control() || matches!(character, '\n' | '\t'))
                && !is_bidi_control(*character)
        })
        .take(MAX_TRANSCRIPT_CHARS)
        .collect()
}

fn normalize_event(mut event: VoiceEvent) -> VoiceEvent {
    event.transcript = event
        .transcript
        .take()
        .map(|transcript| sanitize_transcript(&transcript));
    event.message = sanitize_transcript(&event.message);
    event
}

fn validate_locale(locale: Option<String>) -> Result<Option<String>, String> {
    let Some(locale) = locale else {
        return Ok(None);
    };
    let locale = locale.trim();
    if locale.is_empty() {
        return Ok(None);
    }
    if locale.len() > 64
        || !locale
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("The voice-input locale is invalid.".to_string());
    }
    Ok(Some(locale.to_string()))
}

fn emit_event(event: VoiceEvent) {
    if let Some(app_handle) = APP_HANDLE.get() {
        let _ = app_handle.emit(EVENT_NAME, normalize_event(event));
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::ffi::{c_char, CStr, CString};

    type VoiceEventCallback = unsafe extern "C" fn(*const c_char);

    extern "C" {
        fn ph7_voice_copy_status_json(locale_identifier: *const c_char) -> *mut c_char;
        fn ph7_voice_free_string(value: *mut c_char);
        fn ph7_voice_request_authorization(callback: VoiceEventCallback);
        fn ph7_voice_start(locale_identifier: *const c_char, callback: VoiceEventCallback);
        fn ph7_voice_stop();
    }

    unsafe extern "C" fn receive_event(json: *const c_char) {
        if json.is_null() {
            return;
        }
        let bytes = unsafe { CStr::from_ptr(json) }.to_bytes();
        if bytes.len() > MAX_NATIVE_EVENT_BYTES {
            emit_event(VoiceEvent {
                kind: "error".to_string(),
                message: "The voice transcript exceeded the local safety limit.".to_string(),
                ..VoiceEvent::default()
            });
            return;
        }
        if let Ok(event) = serde_json::from_slice::<VoiceEvent>(bytes) {
            emit_event(event);
        }
    }

    fn locale_c_string(locale: Option<String>) -> Result<CString, String> {
        CString::new(locale.unwrap_or_default())
            .map_err(|_| "The voice-input locale is invalid.".to_string())
    }

    pub fn status(locale: Option<String>) -> Result<VoiceEvent, String> {
        let locale = locale_c_string(locale)?;
        let pointer = unsafe { ph7_voice_copy_status_json(locale.as_ptr()) };
        if pointer.is_null() {
            return Err("Could not read the on-device voice-input status.".to_string());
        }
        let bytes = unsafe { CStr::from_ptr(pointer) }.to_bytes().to_vec();
        unsafe { ph7_voice_free_string(pointer) };
        if bytes.len() > MAX_NATIVE_EVENT_BYTES {
            return Err("The voice-input status exceeded the local safety limit.".to_string());
        }
        serde_json::from_slice::<VoiceEvent>(&bytes)
            .map(normalize_event)
            .map_err(|_| "Could not decode the on-device voice-input status.".to_string())
    }

    pub fn request_access() {
        unsafe { ph7_voice_request_authorization(receive_event) };
    }

    pub fn start(locale: Option<String>) -> Result<(), String> {
        let locale = locale_c_string(locale)?;
        unsafe { ph7_voice_start(locale.as_ptr(), receive_event) };
        Ok(())
    }

    pub fn stop() {
        unsafe { ph7_voice_stop() };
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub fn status(_locale: Option<String>) -> Result<VoiceEvent, String> {
        Ok(VoiceEvent::default())
    }

    pub fn request_access() {
        emit_event(VoiceEvent::default());
    }

    pub fn start(_locale: Option<String>) -> Result<(), String> {
        Err("On-device voice input is currently available only on macOS.".to_string())
    }

    pub fn stop() {}
}

pub fn status(locale: Option<String>) -> Result<VoiceEvent, String> {
    platform::status(validate_locale(locale)?)
}

pub fn request_access() {
    platform::request_access();
}

pub fn start(locale: Option<String>) -> Result<(), String> {
    platform::start(validate_locale(locale)?)
}

pub fn stop() {
    platform::stop();
}

#[cfg(test)]
mod tests {
    use super::{normalize_event, sanitize_transcript, validate_locale, VoiceEvent};

    #[test]
    fn transcript_removes_terminal_controls_and_bidi_overrides() {
        assert_eq!(
            sanitize_transcript("show files\u{1b}[31m\u{202e}safe\nnext"),
            "show files[31msafe\nnext"
        );
    }

    #[test]
    fn transcript_is_bounded_before_crossing_into_the_ui() {
        let event = normalize_event(VoiceEvent {
            transcript: Some("a".repeat(5_000)),
            ..VoiceEvent::default()
        });
        assert_eq!(event.transcript.expect("transcript").chars().count(), 4_000);
    }

    #[test]
    fn locale_validation_accepts_apple_identifiers_only() {
        assert_eq!(
            validate_locale(Some("en-AU".into())).unwrap(),
            Some("en-AU".into())
        );
        assert!(validate_locale(Some("en-AU\0unsafe".into())).is_err());
        assert!(validate_locale(Some("../en".into())).is_err());
    }
}
