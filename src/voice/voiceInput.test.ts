import { describe, expect, it } from 'vitest';
import {
  initialVoiceInputState,
  voiceButtonLabel,
  voiceInputReducer,
} from './voiceInput';

describe('voiceInputReducer', () => {
  it('moves from local availability to listening and back to reviewable idle', () => {
    const ready = voiceInputReducer(initialVoiceInputState, {
      kind: 'status',
      available: true,
      onDeviceAvailable: true,
      microphoneAuthorization: 'authorized',
      speechAuthorization: 'authorized',
    });
    const listening = voiceInputReducer(ready, { kind: 'partial', transcript: 'show large files' });
    const finished = voiceInputReducer(listening, { kind: 'final', transcript: 'show large files' });

    expect(listening.phase).toBe('listening');
    expect(voiceButtonLabel(listening)).toBe('Stop voice input');
    expect(finished.phase).toBe('idle');
  });

  it('distinguishes denied permission from missing on-device recognition', () => {
    const denied = voiceInputReducer(initialVoiceInputState, {
      kind: 'status',
      onDeviceAvailable: true,
      microphoneAuthorization: 'denied',
      speechAuthorization: 'authorized',
    });
    const unavailable = voiceInputReducer(initialVoiceInputState, {
      kind: 'status',
      onDeviceAvailable: false,
      microphoneAuthorization: 'authorized',
      speechAuthorization: 'authorized',
    });

    expect(denied.phase).toBe('denied');
    expect(unavailable.phase).toBe('unavailable');
  });

  it('surfaces native errors without claiming the microphone is active', () => {
    const failed = voiceInputReducer(initialVoiceInputState, {
      kind: 'error',
      message: 'The microphone could not be started.',
    });

    expect(failed.phase).toBe('error');
    expect(failed.message).toContain('microphone');
    expect(voiceButtonLabel(failed)).toBe('Start on-device voice input');
  });
});
