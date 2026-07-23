#import <AVFoundation/AVFoundation.h>
#import <Foundation/Foundation.h>
#import <Speech/Speech.h>

#include <stdlib.h>
#include <string.h>

typedef void (*PH7VoiceEventCallback)(const char *json);

static NSString *PH7SpeechAuthorizationName(SFSpeechRecognizerAuthorizationStatus status) {
  switch (status) {
    case SFSpeechRecognizerAuthorizationStatusAuthorized: return @"authorized";
    case SFSpeechRecognizerAuthorizationStatusDenied: return @"denied";
    case SFSpeechRecognizerAuthorizationStatusRestricted: return @"restricted";
    case SFSpeechRecognizerAuthorizationStatusNotDetermined: return @"notDetermined";
  }
  return @"unknown";
}

static NSString *PH7MicrophoneAuthorizationName(AVAuthorizationStatus status) {
  switch (status) {
    case AVAuthorizationStatusAuthorized: return @"authorized";
    case AVAuthorizationStatusDenied: return @"denied";
    case AVAuthorizationStatusRestricted: return @"restricted";
    case AVAuthorizationStatusNotDetermined: return @"notDetermined";
  }
  return @"unknown";
}

static void PH7Emit(PH7VoiceEventCallback callback, NSDictionary *payload) {
  if (callback == NULL || payload == nil) return;
  NSError *error = nil;
  NSData *data = [NSJSONSerialization dataWithJSONObject:payload options:0 error:&error];
  if (data == nil || error != nil) return;
  NSString *json = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
  if (json != nil) callback(json.UTF8String);
}

static NSLocale *PH7Locale(const char *identifier) {
  if (identifier == NULL || identifier[0] == '\0') return NSLocale.currentLocale;
  NSString *value = [NSString stringWithUTF8String:identifier];
  return value.length == 0 ? NSLocale.currentLocale : [[NSLocale alloc] initWithLocaleIdentifier:value];
}

static NSDictionary *PH7Status(NSLocale *locale) {
  AVAuthorizationStatus microphone = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
  SFSpeechRecognizerAuthorizationStatus speech = SFSpeechRecognizer.authorizationStatus;
  SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc] initWithLocale:locale];
  BOOL onDevice = recognizer != nil && recognizer.supportsOnDeviceRecognition;
  BOOL ready = microphone == AVAuthorizationStatusAuthorized &&
               speech == SFSpeechRecognizerAuthorizationStatusAuthorized &&
               recognizer.available && onDevice;

  NSString *message = @"On-device voice input is ready.";
  if (recognizer == nil || !onDevice) {
    message = @"On-device speech recognition is unavailable for this language on this Mac.";
  } else if (microphone == AVAuthorizationStatusDenied || microphone == AVAuthorizationStatusRestricted ||
             speech == SFSpeechRecognizerAuthorizationStatusDenied || speech == SFSpeechRecognizerAuthorizationStatusRestricted) {
    message = @"Voice input needs Microphone and Speech Recognition access in System Settings.";
  } else if (microphone == AVAuthorizationStatusNotDetermined || speech == SFSpeechRecognizerAuthorizationStatusNotDetermined) {
    message = @"Voice input is available after you grant local microphone and speech access.";
  } else if (!recognizer.available) {
    message = @"On-device speech recognition is temporarily unavailable.";
  }

  return @{
    @"kind": @"status",
    @"available": @(ready),
    @"onDeviceAvailable": @(onDevice),
    @"microphoneAuthorization": PH7MicrophoneAuthorizationName(microphone),
    @"speechAuthorization": PH7SpeechAuthorizationName(speech),
    @"message": message,
  };
}

@interface PH7VoiceController : NSObject
@property(nonatomic, strong) AVAudioEngine *audioEngine;
@property(nonatomic, strong) SFSpeechAudioBufferRecognitionRequest *request;
@property(nonatomic, strong) SFSpeechRecognitionTask *task;
@property(nonatomic, strong) dispatch_source_t timeout;
@property(nonatomic, copy) NSString *latestTranscript;
@property(nonatomic, assign) PH7VoiceEventCallback callback;
@property(nonatomic, assign) NSUInteger generation;
@property(nonatomic, assign) BOOL stopping;
@end

@implementation PH7VoiceController

- (instancetype)init {
  self = [super init];
  if (self) {
    _audioEngine = [[AVAudioEngine alloc] init];
    _latestTranscript = @"";
  }
  return self;
}

- (void)removeInputTapAndStopAudio {
  if (self.audioEngine.running) [self.audioEngine stop];
  @try {
    [self.audioEngine.inputNode removeTapOnBus:0];
  } @catch (__unused NSException *exception) {
    // There is no tap to remove when startup failed before installation.
  }
}

- (void)cancelTimeout {
  if (self.timeout != nil) {
    dispatch_source_cancel(self.timeout);
    self.timeout = nil;
  }
}

- (void)finishGeneration:(NSUInteger)generation transcript:(NSString *)transcript {
  if (generation != self.generation || self.task == nil) return;
  [self cancelTimeout];
  [self removeInputTapAndStopAudio];
  [self.request endAudio];
  [self.task cancel];
  self.task = nil;
  self.request = nil;
  self.stopping = NO;
  NSString *cleanTranscript = transcript ?: self.latestTranscript ?: @"";
  PH7Emit(self.callback, @{
    @"kind": @"final",
    @"transcript": cleanTranscript,
    @"isFinal": @YES,
    @"available": @YES,
    @"onDeviceAvailable": @YES,
    @"message": cleanTranscript.length > 0 ? @"Voice draft is ready for review." : @"No speech was detected.",
  });
}

- (void)stopWithFinalResult {
  if (self.task == nil) return;
  self.stopping = YES;
  NSUInteger generation = self.generation;
  [self cancelTimeout];
  [self removeInputTapAndStopAudio];
  [self.request endAudio];
  PH7Emit(self.callback, @{
    @"kind": @"processing",
    @"available": @YES,
    @"onDeviceAvailable": @YES,
    @"message": @"Finishing the on-device transcript…",
  });

  dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(2 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
    [self finishGeneration:generation transcript:self.latestTranscript];
  });
}

- (void)startWithLocale:(NSLocale *)locale callback:(PH7VoiceEventCallback)callback {
  self.callback = callback;
  if (self.task != nil) {
    PH7Emit(callback, @{
      @"kind": @"error",
      @"available": @YES,
      @"onDeviceAvailable": @YES,
      @"message": @"Voice input is already active.",
    });
    return;
  }

  NSDictionary *status = PH7Status(locale);
  if (![status[@"available"] boolValue]) {
    PH7Emit(callback, status);
    return;
  }

  SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc] initWithLocale:locale];
  if (recognizer == nil || !recognizer.supportsOnDeviceRecognition) {
    PH7Emit(callback, status);
    return;
  }

  self.generation += 1;
  NSUInteger generation = self.generation;
  self.stopping = NO;
  self.latestTranscript = @"";
  self.request = [[SFSpeechAudioBufferRecognitionRequest alloc] init];
  self.request.requiresOnDeviceRecognition = YES;
  self.request.shouldReportPartialResults = YES;
  self.request.taskHint = SFSpeechRecognitionTaskHintDictation;
  if ([self.request respondsToSelector:@selector(setAddsPunctuation:)]) {
    self.request.addsPunctuation = YES;
  }
  self.request.contextualStrings = @[@"git", @"npm", @"cargo", @"rustup", @"docker", @"kubectl", @"localhost"];

  __weak PH7VoiceController *weakSelf = self;
  self.task = [recognizer recognitionTaskWithRequest:self.request resultHandler:^(SFSpeechRecognitionResult *result, NSError *error) {
    dispatch_async(dispatch_get_main_queue(), ^{
      PH7VoiceController *strongSelf = weakSelf;
      if (strongSelf == nil || generation != strongSelf.generation || strongSelf.task == nil) return;

      if (result != nil) {
        NSString *transcript = result.bestTranscription.formattedString ?: @"";
        strongSelf.latestTranscript = transcript;
        if (result.final) {
          [strongSelf finishGeneration:generation transcript:transcript];
          return;
        }
        PH7Emit(strongSelf.callback, @{
          @"kind": @"partial",
          @"transcript": transcript,
          @"isFinal": @NO,
          @"available": @YES,
          @"onDeviceAvailable": @YES,
          @"message": @"Listening on this Mac…",
        });
      }

      if (error != nil) {
        if (strongSelf.stopping) {
          [strongSelf finishGeneration:generation transcript:strongSelf.latestTranscript];
          return;
        }
        [strongSelf cancelTimeout];
        [strongSelf removeInputTapAndStopAudio];
        strongSelf.task = nil;
        strongSelf.request = nil;
        PH7Emit(strongSelf.callback, @{
          @"kind": @"error",
          @"available": @YES,
          @"onDeviceAvailable": @YES,
          @"message": @"On-device speech recognition stopped. Try again in a quieter environment.",
        });
      }
    });
  }];

  AVAudioInputNode *input = self.audioEngine.inputNode;
  AVAudioFormat *format = [input outputFormatForBus:0];
  if (format.sampleRate <= 0 || format.channelCount == 0) {
    [self.task cancel];
    self.task = nil;
    self.request = nil;
    PH7Emit(callback, @{
      @"kind": @"error",
      @"available": @YES,
      @"onDeviceAvailable": @YES,
      @"message": @"No usable microphone input is available.",
    });
    return;
  }

  [input installTapOnBus:0 bufferSize:1024 format:format block:^(AVAudioPCMBuffer *buffer, __unused AVAudioTime *when) {
    PH7VoiceController *strongSelf = weakSelf;
    if (strongSelf != nil && generation == strongSelf.generation) {
      [strongSelf.request appendAudioPCMBuffer:buffer];
    }
  }];

  NSError *audioError = nil;
  [self.audioEngine prepare];
  if (![self.audioEngine startAndReturnError:&audioError]) {
    [self removeInputTapAndStopAudio];
    [self.task cancel];
    self.task = nil;
    self.request = nil;
    PH7Emit(callback, @{
      @"kind": @"error",
      @"available": @YES,
      @"onDeviceAvailable": @YES,
      @"message": @"The microphone could not be started.",
    });
    return;
  }

  PH7Emit(callback, @{
    @"kind": @"listening",
    @"available": @YES,
    @"onDeviceAvailable": @YES,
    @"message": @"Listening on this Mac…",
  });

  self.timeout = dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, 0, dispatch_get_main_queue());
  dispatch_source_set_timer(self.timeout, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC), DISPATCH_TIME_FOREVER, NSEC_PER_SEC / 10);
  dispatch_source_set_event_handler(self.timeout, ^{
    PH7VoiceController *strongSelf = weakSelf;
    if (strongSelf != nil && generation == strongSelf.generation) [strongSelf stopWithFinalResult];
  });
  dispatch_resume(self.timeout);
}

@end

static PH7VoiceController *PH7Controller(void) {
  static PH7VoiceController *controller;
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{ controller = [[PH7VoiceController alloc] init]; });
  return controller;
}

char *ph7_voice_copy_status_json(const char *locale_identifier) {
  NSDictionary *status = PH7Status(PH7Locale(locale_identifier));
  NSData *data = [NSJSONSerialization dataWithJSONObject:status options:0 error:nil];
  NSString *json = data == nil ? nil : [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
  return json == nil ? NULL : strdup(json.UTF8String);
}

void ph7_voice_free_string(char *value) {
  free(value);
}

void ph7_voice_request_authorization(PH7VoiceEventCallback callback) {
  dispatch_async(dispatch_get_main_queue(), ^{
    dispatch_group_t group = dispatch_group_create();
    dispatch_group_enter(group);
    [SFSpeechRecognizer requestAuthorization:^(__unused SFSpeechRecognizerAuthorizationStatus status) {
      dispatch_group_leave(group);
    }];
    dispatch_group_enter(group);
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(__unused BOOL granted) {
      dispatch_group_leave(group);
    }];
    dispatch_group_notify(group, dispatch_get_main_queue(), ^{
      PH7Emit(callback, PH7Status(NSLocale.currentLocale));
    });
  });
}

void ph7_voice_start(const char *locale_identifier, PH7VoiceEventCallback callback) {
  NSString *identifier = locale_identifier == NULL ? nil : [NSString stringWithUTF8String:locale_identifier];
  dispatch_async(dispatch_get_main_queue(), ^{
    NSLocale *locale = identifier.length == 0 ? NSLocale.currentLocale : [[NSLocale alloc] initWithLocaleIdentifier:identifier];
    [PH7Controller() startWithLocale:locale callback:callback];
  });
}

void ph7_voice_stop(void) {
  dispatch_async(dispatch_get_main_queue(), ^{ [PH7Controller() stopWithFinalResult]; });
}
