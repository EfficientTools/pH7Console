export type VoiceInputPhase =
  | 'checking'
  | 'requesting'
  | 'idle'
  | 'listening'
  | 'processing'
  | 'denied'
  | 'unavailable'
  | 'error';

export interface VoiceInputEvent {
  kind: 'status' | 'partial' | 'final' | 'listening' | 'processing' | 'error' | 'requesting';
  transcript?: string;
  isFinal?: boolean;
  available?: boolean;
  onDeviceAvailable?: boolean;
  microphoneAuthorization?: string;
  speechAuthorization?: string;
  message?: string;
}

export interface VoiceInputState {
  phase: VoiceInputPhase;
  available: boolean;
  onDeviceAvailable: boolean;
  microphoneAuthorization: string;
  speechAuthorization: string;
  message: string;
}

export const initialVoiceInputState: VoiceInputState = {
  phase: 'checking',
  available: false,
  onDeviceAvailable: false,
  microphoneAuthorization: 'unknown',
  speechAuthorization: 'unknown',
  message: 'Checking on-device voice input…',
};

const permissionIsDenied = (authorization: string) =>
  authorization === 'denied' || authorization === 'restricted';

export const voiceInputReducer = (
  state: VoiceInputState,
  event: VoiceInputEvent,
): VoiceInputState => {
  const next = {
    ...state,
    available: event.available ?? state.available,
    onDeviceAvailable: event.onDeviceAvailable ?? state.onDeviceAvailable,
    microphoneAuthorization: event.microphoneAuthorization ?? state.microphoneAuthorization,
    speechAuthorization: event.speechAuthorization ?? state.speechAuthorization,
    message: event.message ?? state.message,
  };

  switch (event.kind) {
    case 'requesting':
      return { ...next, phase: 'requesting', message: event.message ?? 'Waiting for permission…' };
    case 'listening':
    case 'partial':
      return { ...next, phase: 'listening' };
    case 'processing':
      return { ...next, phase: 'processing' };
    case 'final':
      return { ...next, phase: 'idle', available: true, onDeviceAvailable: true };
    case 'error':
      return { ...next, phase: 'error' };
    case 'status': {
      if (
        permissionIsDenied(next.microphoneAuthorization) ||
        permissionIsDenied(next.speechAuthorization)
      ) {
        return { ...next, phase: 'denied' };
      }
      if (!next.onDeviceAvailable) return { ...next, phase: 'unavailable' };
      return { ...next, phase: 'idle' };
    }
  }
};

export const voiceButtonLabel = (state: VoiceInputState): string => {
  switch (state.phase) {
    case 'listening': return 'Stop voice input';
    case 'processing': return 'Finishing voice transcript';
    case 'requesting': return 'Waiting for voice permission';
    case 'denied': return 'Voice permission denied';
    case 'unavailable': return 'On-device voice unavailable';
    default: return 'Start on-device voice input';
  }
};
