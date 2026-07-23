//! Genuine, local-only LLM inference through a managed `llama-server` process.
//!
//! This module deliberately does not contain a cloud provider. It accepts only
//! HTTP endpoints whose resolved address is loopback, verifies the llama.cpp
//! health and model-list APIs before use, and streams chat-completion tokens.
//! A production build should bundle and sign `llama-server`; callers may also
//! provide explicit, verified loopback servers for development or power users.
//!
//! The module uses only dependencies already present in this crate (Tokio,
//! Serde, serde_json, and UUID), and remains compatible with Rust 1.77.2.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Notify};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

const MAX_HTTP_HEAD_BYTES: usize = 64 * 1024;
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;
const MAX_JSON_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmErrorKind {
    Configuration,
    Unavailable,
    Authentication,
    InvalidRequest,
    Protocol,
    Server,
    Cancelled,
    Io,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmError {
    pub kind: LlmErrorKind,
    pub message: String,
}

impl LlmError {
    fn new(kind: LlmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn configuration(message: impl Into<String>) -> Self {
        Self::new(LlmErrorKind::Configuration, message)
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self::new(LlmErrorKind::Unavailable, message)
    }

    fn protocol(message: impl Into<String>) -> Self {
        Self::new(LlmErrorKind::Protocol, message)
    }
}

impl fmt::Display for LlmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for LlmError {}

impl From<io::Error> for LlmError {
    fn from(error: io::Error) -> Self {
        Self::new(LlmErrorKind::Io, error.to_string())
    }
}

/// An HTTP endpoint guaranteed to resolve to this machine without DNS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoopbackEndpoint {
    address: SocketAddr,
}

impl LoopbackEndpoint {
    pub fn new(address: SocketAddr) -> Result<Self, LlmError> {
        if !address.ip().is_loopback() {
            return Err(LlmError::configuration(
                "Local LLM endpoints must use a loopback address",
            ));
        }
        if address.port() == 0 {
            return Err(LlmError::configuration(
                "Local LLM endpoint port must be non-zero",
            ));
        }
        Ok(Self { address })
    }

    /// Parse an endpoint root such as `http://127.0.0.1:8080`.
    ///
    /// `localhost` is mapped directly to 127.0.0.1; no DNS lookup occurs.
    /// Paths, query strings, user information, HTTPS, and non-loopback hosts
    /// are rejected intentionally.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn parse(input: &str) -> Result<Self, LlmError> {
        let authority = input
            .strip_prefix("http://")
            .ok_or_else(|| LlmError::configuration("Local LLM endpoint must use http://"))?
            .trim_end_matches('/');

        if authority.is_empty()
            || authority.contains('/')
            || authority.contains('?')
            || authority.contains('#')
            || authority.contains('@')
        {
            return Err(LlmError::configuration(
                "Local LLM endpoint must be an HTTP loopback origin without a path",
            ));
        }

        let address = if let Some(bracketed) = authority.strip_prefix('[') {
            let end = bracketed
                .find(']')
                .ok_or_else(|| LlmError::configuration("Invalid bracketed IPv6 endpoint"))?;
            let host = &bracketed[..end];
            let port = bracketed[end + 1..]
                .strip_prefix(':')
                .ok_or_else(|| LlmError::configuration("Local LLM endpoint requires a port"))?;
            SocketAddr::new(
                IpAddr::V6(
                    host.parse::<Ipv6Addr>()
                        .map_err(|_| LlmError::configuration("Invalid IPv6 endpoint"))?,
                ),
                parse_port(port)?,
            )
        } else {
            let (host, port) = authority
                .rsplit_once(':')
                .ok_or_else(|| LlmError::configuration("Local LLM endpoint requires a port"))?;
            let ip = if host == "localhost" {
                IpAddr::V4(Ipv4Addr::LOCALHOST)
            } else {
                host.parse::<IpAddr>()
                    .map_err(|_| LlmError::configuration("Endpoint host must be a loopback IP"))?
            };
            SocketAddr::new(ip, parse_port(port)?)
        };

        Self::new(address)
    }

    pub fn address(self) -> SocketAddr {
        self.address
    }

    pub fn origin(self) -> String {
        format!("http://{}", self.authority())
    }

    fn authority(self) -> String {
        match self.address.ip() {
            IpAddr::V4(ip) => format!("{}:{}", ip, self.address.port()),
            IpAddr::V6(ip) => format!("[{}]:{}", ip, self.address.port()),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_port(value: &str) -> Result<u16, LlmError> {
    let port = value
        .parse::<u16>()
        .map_err(|_| LlmError::configuration("Invalid local LLM endpoint port"))?;
    if port == 0 {
        return Err(LlmError::configuration(
            "Local LLM endpoint port must be non-zero",
        ));
    }
    Ok(port)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub seed: Option<u64>,
    pub stop: Vec<String>,
    /// Optional llama.cpp/OpenAI-compatible response format, including a JSON
    /// schema. The caller remains responsible for validating generated output.
    pub response_format: Option<Value>,
}

impl GenerationRequest {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn terminal_assistant(prompt: impl Into<String>) -> Self {
        Self {
            model: None,
            messages: vec![ChatMessage::user(prompt)],
            max_tokens: 512,
            temperature: 0.2,
            top_p: 0.9,
            seed: None,
            stop: Vec::new(),
            response_format: None,
        }
    }

    fn validate(&self) -> Result<(), LlmError> {
        if self.messages.is_empty() {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "At least one chat message is required",
            ));
        }
        if self.max_tokens == 0 || self.max_tokens > 16_384 {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "max_tokens must be between 1 and 16384",
            ));
        }
        if !self.temperature.is_finite() || !(0.0..=2.0).contains(&self.temperature) {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "temperature must be finite and between 0 and 2",
            ));
        }
        if !self.top_p.is_finite() || !(0.0..=1.0).contains(&self.top_p) || self.top_p == 0.0 {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "top_p must be finite and greater than 0 and at most 1",
            ));
        }
        if self
            .messages
            .iter()
            .any(|message| message.content.is_empty())
        {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "Chat messages may not be empty",
            ));
        }
        if self
            .messages
            .iter()
            .map(|message| message.content.len())
            .sum::<usize>()
            > MAX_REQUEST_BYTES
        {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "Chat request is too large",
            ));
        }
        if let Some(model) = self.model.as_deref() {
            validate_header_safe_value("model", model, 512)?;
        }
        if self.stop.len() > 32
            || self
                .stop
                .iter()
                .any(|stop| stop.len() > 1024 || stop.contains('\0'))
        {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "Invalid stop sequence configuration",
            ));
        }
        if self
            .response_format
            .as_ref()
            .and_then(|value| serde_json::to_vec(value).ok())
            .is_some_and(|value| value.len() > 256 * 1024)
        {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "Response schema is too large",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GenerationMetrics {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub prompt_tokens_per_second: Option<f64>,
    pub generated_tokens_per_second: Option<f64>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Started {
        request_id: Option<String>,
        model: Option<String>,
    },
    Delta {
        text: String,
    },
    Finished {
        metrics: GenerationMetrics,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendSource {
    BundledLlamaServer,
    VerifiedLoopback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderInfo {
    pub source: BackendSource,
    pub endpoint: String,
    pub model_ids: Vec<String>,
    pub managed_process: bool,
}

type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, LlmError>> + Send + 'a>>;

/// Backend abstraction kept intentionally narrow so an embedded engine can be
/// added later without changing the application's AI orchestration layer.
pub trait LocalLlmProvider: Send + Sync {
    fn info(&self) -> ProviderInfo;
    fn verify(&self) -> ProviderFuture<'_, ProviderInfo>;
    fn start_stream(&self, request: GenerationRequest) -> Result<GenerationStream, LlmError>;
}

#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<CancellationState>,
}

struct CancellationState {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationState {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    pub fn cancel(&self) {
        if !self.inner.cancelled.swap(true, Ordering::SeqCst) {
            self.inner.notify.notify_waiters();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let notified = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GenerationStream {
    receiver: mpsc::Receiver<Result<StreamEvent, LlmError>>,
    cancellation: CancellationToken,
}

impl GenerationStream {
    pub async fn recv(&mut self) -> Option<Result<StreamEvent, LlmError>> {
        self.receiver.recv().await
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
}

impl Drop for GenerationStream {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

#[derive(Clone)]
pub struct LoopbackLlamaServer {
    endpoint: LoopbackEndpoint,
    api_key: Option<Arc<str>>,
    default_model: Arc<Mutex<Option<String>>>,
    source: BackendSource,
    managed_process: bool,
    connect_timeout: Duration,
    idle_timeout: Duration,
}

impl fmt::Debug for LoopbackLlamaServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoopbackLlamaServer")
            .field("endpoint", &self.endpoint)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("source", &self.source)
            .field("managed_process", &self.managed_process)
            .finish()
    }
}

impl LoopbackLlamaServer {
    pub fn new(
        endpoint: LoopbackEndpoint,
        api_key: Option<String>,
        default_model: Option<String>,
    ) -> Result<Self, LlmError> {
        Self::with_source(
            endpoint,
            api_key,
            default_model,
            BackendSource::VerifiedLoopback,
            false,
        )
    }

    fn with_source(
        endpoint: LoopbackEndpoint,
        api_key: Option<String>,
        default_model: Option<String>,
        source: BackendSource,
        managed_process: bool,
    ) -> Result<Self, LlmError> {
        if let Some(key) = api_key.as_deref() {
            validate_header_safe_value("API key", key, 4096)?;
        }
        if let Some(model) = default_model.as_deref() {
            validate_header_safe_value("model", model, 512)?;
        }

        Ok(Self {
            endpoint,
            api_key: api_key.map(Arc::from),
            default_model: Arc::new(Mutex::new(default_model)),
            source,
            managed_process,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
        })
    }

    pub fn with_timeouts(mut self, connect_timeout: Duration, idle_timeout: Duration) -> Self {
        self.connect_timeout = connect_timeout;
        self.idle_timeout = idle_timeout;
        self
    }

    async fn verify_server(&self) -> Result<ProviderInfo, LlmError> {
        let cancellation = CancellationToken::new();
        let health = self
            .json_request("GET", "/health", None, &cancellation)
            .await?;
        if health.get("status").and_then(Value::as_str) != Some("ok") {
            return Err(LlmError::protocol(
                "Loopback service did not return the llama.cpp health response",
            ));
        }

        let models = self
            .json_request("GET", "/v1/models", None, &cancellation)
            .await?;
        let model_ids: Vec<String> = models
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                LlmError::protocol("Loopback service does not expose a compatible model list")
            })?
            .iter()
            .filter_map(|model| model.get("id").and_then(Value::as_str))
            .map(str::to_owned)
            .collect();

        if model_ids.is_empty() {
            return Err(LlmError::unavailable(
                "The local inference server has no available model",
            ));
        }

        let mut default_model = lock_unpoisoned(&self.default_model);
        if default_model.is_none() {
            *default_model = model_ids.first().cloned();
        }
        drop(default_model);

        Ok(ProviderInfo {
            source: self.source,
            endpoint: self.endpoint.origin(),
            model_ids,
            managed_process: self.managed_process,
        })
    }

    async fn json_request(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        cancellation: &CancellationToken,
    ) -> Result<Value, LlmError> {
        let mut response = open_http_request(
            HttpRequest {
                endpoint: self.endpoint,
                method,
                path,
                body: body.as_deref(),
                api_key: self.api_key.as_deref(),
                connect_timeout: self.connect_timeout,
                idle_timeout: self.idle_timeout,
            },
            cancellation,
        )
        .await?;

        let status = response.status;
        let body = response
            .body
            .read_all(cancellation, MAX_JSON_BODY_BYTES)
            .await?;
        if !(200..300).contains(&status) {
            return Err(http_status_error(status, &body));
        }
        serde_json::from_slice(&body)
            .map_err(|error| LlmError::protocol(format!("Invalid local JSON response: {error}")))
    }

    async fn run_stream(
        &self,
        request: GenerationRequest,
        cancellation: CancellationToken,
        sender: mpsc::Sender<Result<StreamEvent, LlmError>>,
    ) -> Result<(), LlmError> {
        request.validate()?;
        let model = request
            .model
            .clone()
            .or_else(|| lock_unpoisoned(&self.default_model).clone())
            .unwrap_or_else(|| "local-model".to_string());

        let mut payload = json!({
            "model": model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": true,
            "stream_options": { "include_usage": true },
            "cache_prompt": true
        });
        if let Some(seed) = request.seed {
            payload["seed"] = json!(seed);
        }
        if !request.stop.is_empty() {
            payload["stop"] = json!(request.stop);
        }
        if let Some(response_format) = request.response_format {
            payload["response_format"] = response_format;
        }
        let body = serde_json::to_vec(&payload).map_err(|error| {
            LlmError::protocol(format!("Could not encode LLM request: {error}"))
        })?;
        if body.len() > MAX_REQUEST_BYTES {
            return Err(LlmError::new(
                LlmErrorKind::InvalidRequest,
                "Encoded chat request is too large",
            ));
        }

        let mut response = open_http_request(
            HttpRequest {
                endpoint: self.endpoint,
                method: "POST",
                path: "/v1/chat/completions",
                body: Some(&body),
                api_key: self.api_key.as_deref(),
                connect_timeout: self.connect_timeout,
                idle_timeout: self.idle_timeout,
            },
            &cancellation,
        )
        .await?;

        if !(200..300).contains(&response.status) {
            let status = response.status;
            let body = response
                .body
                .read_all(&cancellation, MAX_ERROR_BODY_BYTES)
                .await?;
            return Err(http_status_error(status, &body));
        }

        let mut sse = SseDecoder::default();
        let mut started = false;
        let mut finished = false;
        let mut metrics = GenerationMetrics::default();

        while let Some(bytes) = response.body.next_data(&cancellation).await? {
            for event in sse.push(&bytes)? {
                if event == b"[DONE]" {
                    finished = true;
                    break;
                }
                let value: Value = serde_json::from_slice(&event).map_err(|error| {
                    LlmError::protocol(format!("Invalid local inference event: {error}"))
                })?;
                if let Some(error) = value.get("error") {
                    let message = error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Local inference failed");
                    return Err(LlmError::new(LlmErrorKind::Server, message));
                }

                if !started {
                    started = true;
                    send_stream_event(
                        &sender,
                        &cancellation,
                        StreamEvent::Started {
                            request_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
                            model: value
                                .get("model")
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                        },
                    )
                    .await?;
                }

                update_metrics(&mut metrics, &value);
                if let Some(text) = value
                    .pointer("/choices/0/delta/content")
                    .and_then(Value::as_str)
                {
                    if !text.is_empty() {
                        send_stream_event(
                            &sender,
                            &cancellation,
                            StreamEvent::Delta {
                                text: text.to_string(),
                            },
                        )
                        .await?;
                    }
                }
            }
            if finished {
                break;
            }
        }

        if !finished {
            for event in sse.finish()? {
                if event != b"[DONE]" {
                    return Err(LlmError::protocol(
                        "Local inference stream ended with an incomplete event",
                    ));
                }
            }
        }

        if cancellation.is_cancelled() {
            return Err(LlmError::new(
                LlmErrorKind::Cancelled,
                "Generation cancelled",
            ));
        }
        send_stream_event(&sender, &cancellation, StreamEvent::Finished { metrics }).await
    }
}

impl LocalLlmProvider for LoopbackLlamaServer {
    fn info(&self) -> ProviderInfo {
        let model_ids = lock_unpoisoned(&self.default_model)
            .clone()
            .into_iter()
            .collect();
        ProviderInfo {
            source: self.source,
            endpoint: self.endpoint.origin(),
            model_ids,
            managed_process: self.managed_process,
        }
    }

    fn verify(&self) -> ProviderFuture<'_, ProviderInfo> {
        Box::pin(self.verify_server())
    }

    fn start_stream(&self, request: GenerationRequest) -> Result<GenerationStream, LlmError> {
        tokio::runtime::Handle::try_current()
            .map_err(|_| LlmError::configuration("Local LLM streaming requires a Tokio runtime"))?;
        request.validate()?;

        let (sender, receiver) = mpsc::channel(64);
        let cancellation = CancellationToken::new();
        let task_cancellation = cancellation.clone();
        let provider = self.clone();
        tokio::spawn(async move {
            if let Err(error) = provider
                .run_stream(request, task_cancellation.clone(), sender.clone())
                .await
            {
                let _ = sender.send(Err(error)).await;
            }
        });

        Ok(GenerationStream {
            receiver,
            cancellation,
        })
    }
}

async fn send_stream_event(
    sender: &mpsc::Sender<Result<StreamEvent, LlmError>>,
    cancellation: &CancellationToken,
    event: StreamEvent,
) -> Result<(), LlmError> {
    tokio::select! {
        _ = cancellation.cancelled() => {
            Err(LlmError::new(LlmErrorKind::Cancelled, "Generation cancelled"))
        }
        result = sender.send(Ok(event)) => {
            result.map_err(|_| LlmError::new(LlmErrorKind::Cancelled, "Generation receiver closed"))
        }
    }
}

fn update_metrics(metrics: &mut GenerationMetrics, value: &Value) {
    if let Some(reason) = value
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
    {
        metrics.finish_reason = Some(reason.to_string());
    }
    if let Some(usage) = value.get("usage") {
        metrics.prompt_tokens = usage.get("prompt_tokens").and_then(Value::as_u64);
        metrics.completion_tokens = usage.get("completion_tokens").and_then(Value::as_u64);
        metrics.total_tokens = usage.get("total_tokens").and_then(Value::as_u64);
    }
    if let Some(timings) = value.get("timings") {
        metrics.prompt_tokens_per_second = timings.get("prompt_per_second").and_then(Value::as_f64);
        metrics.generated_tokens_per_second =
            timings.get("predicted_per_second").and_then(Value::as_f64);
    }
}

#[derive(Clone)]
pub struct LoopbackServerConfig {
    pub endpoint: LoopbackEndpoint,
    pub api_key: Option<String>,
    pub default_model: Option<String>,
}

impl fmt::Debug for LoopbackServerConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoopbackServerConfig")
            .field("endpoint", &self.endpoint)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("default_model", &self.default_model)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Existing GGUF file. Model download and integrity verification belong in
    /// a separate artifact manager; executable downloads are never supported.
    pub model_path: Option<PathBuf>,
    pub bundled_server_candidates: Vec<PathBuf>,
    pub loopback_candidates: Vec<LoopbackServerConfig>,
    pub context_tokens: u32,
    pub parallel_sequences: u16,
    pub startup_timeout: Duration,
    pub connect_timeout: Duration,
    pub generation_idle_timeout: Duration,
}

impl RuntimeConfig {
    pub fn local_defaults(model_path: Option<PathBuf>) -> Self {
        Self {
            model_path,
            bundled_server_candidates: standard_bundle_candidates(),
            // An HTTP shape check cannot authenticate an unrelated local
            // process. External loopback providers therefore require an
            // explicit trusted configuration instead of silently probing a
            // conventional port and sending terminal context to it.
            loopback_candidates: Vec::new(),
            context_tokens: 8192,
            // Application requests are intentionally serialized to protect
            // interactive terminal responsiveness. A second llama.cpp slot
            // would halve the per-sequence context and reserve needless KV
            // cache on memory-constrained Macs.
            parallel_sequences: 1,
            startup_timeout: Duration::from_secs(120),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            generation_idle_timeout: DEFAULT_IDLE_TIMEOUT,
        }
    }
}

/// Owns a verified provider and, when applicable, its managed sidecar process.
pub struct RealLocalLlm {
    provider: LoopbackLlamaServer,
    child: Option<ManagedChild>,
}

impl fmt::Debug for RealLocalLlm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RealLocalLlm")
            .field("provider", &self.provider)
            .field("managed_process", &self.child.is_some())
            .finish()
    }
}

impl RealLocalLlm {
    pub async fn connect(config: RuntimeConfig) -> Result<Self, LlmError> {
        validate_runtime_config(&config)?;
        let mut failures = Vec::new();

        if let Some(model_path) = config.model_path.as_deref() {
            for executable in config
                .bundled_server_candidates
                .iter()
                .filter(|path| path.is_file())
            {
                match launch_bundled_server(executable, model_path, &config).await {
                    Ok(runtime) => return Ok(runtime),
                    Err(error) => failures.push(format!("{}: {}", executable.display(), error)),
                }
            }
        }

        for candidate in &config.loopback_candidates {
            let provider = LoopbackLlamaServer::new(
                candidate.endpoint,
                candidate.api_key.clone(),
                candidate.default_model.clone(),
            )?
            .with_timeouts(config.connect_timeout, config.generation_idle_timeout);
            match provider.verify().await {
                Ok(_) => {
                    return Ok(Self {
                        provider,
                        child: None,
                    })
                }
                Err(error) => failures.push(format!("{}: {}", candidate.endpoint.origin(), error)),
            }
        }

        let detail = if failures.is_empty() {
            "No bundled llama-server or loopback provider was configured".to_string()
        } else {
            format!("No usable local inference backend: {}", failures.join("; "))
        };
        Err(LlmError::unavailable(detail))
    }

    pub fn provider(&self) -> &LoopbackLlamaServer {
        &self.provider
    }

    /// A managed sidecar can exit after its initial health check. Inspecting
    /// the child before status reporting or generation prevents the UI from
    /// advertising a stale ready state. Explicit loopback providers have no
    /// child handle and are checked by their next request instead.
    pub fn ensure_managed_process_running(&mut self) -> Result<(), LlmError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        match child.try_wait().map_err(LlmError::from)? {
            Some(status) => Err(LlmError::unavailable(format!(
                "Bundled llama-server exited with {status}"
            ))),
            None => Ok(()),
        }
    }
}

fn validate_runtime_config(config: &RuntimeConfig) -> Result<(), LlmError> {
    if config.context_tokens < 512 || config.context_tokens > 1_048_576 {
        return Err(LlmError::configuration(
            "context_tokens must be between 512 and 1048576",
        ));
    }
    if config.parallel_sequences == 0 || config.parallel_sequences > 64 {
        return Err(LlmError::configuration(
            "parallel_sequences must be between 1 and 64",
        ));
    }
    if config.startup_timeout.is_zero()
        || config.connect_timeout.is_zero()
        || config.generation_idle_timeout.is_zero()
    {
        return Err(LlmError::configuration("LLM timeouts must be non-zero"));
    }
    if let Some(model_path) = config.model_path.as_deref() {
        validate_model_path(model_path)?;
    }
    Ok(())
}

fn validate_model_path(path: &Path) -> Result<(), LlmError> {
    let metadata = path.metadata().map_err(|error| {
        LlmError::configuration(format!("Cannot read model {}: {error}", path.display()))
    })?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(LlmError::configuration(
            "The configured GGUF model must be a non-empty regular file",
        ));
    }
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("gguf"))
    {
        return Err(LlmError::configuration(
            "The configured local model must be a GGUF file",
        ));
    }
    Ok(())
}

async fn launch_bundled_server(
    executable: &Path,
    model_path: &Path,
    config: &RuntimeConfig,
) -> Result<RealLocalLlm, LlmError> {
    validate_executable(executable)?;
    let listener = StdTcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(LlmError::from)?;
    let port = listener.local_addr().map_err(LlmError::from)?.port();
    drop(listener);

    let api_key = format!("ph7-{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let child = Command::new(executable)
        .arg("--model")
        .arg(model_path)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--ctx-size")
        .arg(config.context_tokens.to_string())
        .arg("--parallel")
        .arg(config.parallel_sequences.to_string())
        .arg("--no-ui")
        .env("LLAMA_API_KEY", &api_key)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            LlmError::unavailable(format!("Could not launch bundled llama-server: {error}"))
        })?;
    // std::process::Child does not terminate its process when dropped. Wrap
    // immediately so cancellation of this async startup future (including an
    // app quit during model warm-up) cannot orphan a resident llama-server.
    let mut child = ManagedChild::new(child);

    let endpoint = LoopbackEndpoint::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))?;
    let provider = LoopbackLlamaServer::with_source(
        endpoint,
        Some(api_key),
        None,
        BackendSource::BundledLlamaServer,
        true,
    )?
    .with_timeouts(config.connect_timeout, config.generation_idle_timeout);

    let started = Instant::now();
    let mut delay = Duration::from_millis(75);
    while started.elapsed() < config.startup_timeout {
        if let Some(status) = child.try_wait().map_err(LlmError::from)? {
            return Err(LlmError::unavailable(format!(
                "Bundled llama-server exited during startup with {status}"
            )));
        }
        let remaining = config.startup_timeout.saturating_sub(started.elapsed());
        let verification = match timeout(remaining, provider.verify()).await {
            Ok(result) => result,
            Err(_) => break,
        };
        match verification {
            Ok(_) => {
                return Ok(RealLocalLlm {
                    provider,
                    child: Some(child),
                })
            }
            Err(error)
                if matches!(
                    error.kind,
                    LlmErrorKind::Unavailable | LlmErrorKind::Io | LlmErrorKind::Server
                ) => {}
            Err(error) => {
                return Err(error);
            }
        }
        sleep(delay).await;
        delay = (delay * 2).min(Duration::from_millis(750));
    }

    Err(LlmError::unavailable(
        "Bundled llama-server did not become ready before the startup timeout",
    ))
}

fn validate_executable(path: &Path) -> Result<(), LlmError> {
    let metadata = path.metadata().map_err(|error| {
        LlmError::configuration(format!("Cannot read sidecar {}: {error}", path.display()))
    })?;
    if !metadata.is_file() {
        return Err(LlmError::configuration(
            "Bundled llama-server must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(LlmError::configuration(
                "Bundled llama-server is not executable",
            ));
        }
    }
    Ok(())
}

struct ManagedChild(Option<Child>);

impl ManagedChild {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        self.0
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "managed child is absent"))?
            .try_wait()
    }

    fn terminate(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn standard_bundle_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(macos_directory) = executable.parent() {
            candidates.push(macos_directory.join("llama-server"));
            if let Some(contents_directory) = macos_directory.parent() {
                candidates.push(contents_directory.join("Helpers/llama-server"));
                candidates.push(contents_directory.join("Resources/llama-server"));
            }
        }
    }

    #[cfg(debug_assertions)]
    if let Ok(current_directory) = std::env::current_dir() {
        let target = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "aarch64-apple-darwin"
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            "x86_64-apple-darwin"
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            "x86_64-pc-windows-msvc"
        } else {
            ""
        };
        if !target.is_empty() {
            let extension = if cfg!(windows) { ".exe" } else { "" };
            candidates.push(
                current_directory
                    .join("src-tauri/binaries")
                    .join(format!("llama-server-{target}{extension}")),
            );
        }
    }

    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn validate_header_safe_value(name: &str, value: &str, max_length: usize) -> Result<(), LlmError> {
    if value.is_empty()
        || value.len() > max_length
        || value
            .bytes()
            .any(|byte| byte == b'\r' || byte == b'\n' || byte == 0)
    {
        return Err(LlmError::configuration(format!("Invalid {name}")));
    }
    Ok(())
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct HttpResponse {
    status: u16,
    body: HttpBody,
}

struct HttpRequest<'a> {
    endpoint: LoopbackEndpoint,
    method: &'a str,
    path: &'a str,
    body: Option<&'a [u8]>,
    api_key: Option<&'a str>,
    connect_timeout: Duration,
    idle_timeout: Duration,
}

async fn open_http_request(
    request: HttpRequest<'_>,
    cancellation: &CancellationToken,
) -> Result<HttpResponse, LlmError> {
    let HttpRequest {
        endpoint,
        method,
        path,
        body,
        api_key,
        connect_timeout,
        idle_timeout,
    } = request;
    if !path.starts_with('/') || path.contains('\r') || path.contains('\n') {
        return Err(LlmError::configuration("Invalid local inference API path"));
    }
    let connect = timeout(connect_timeout, TcpStream::connect(endpoint.address()));
    let mut stream = tokio::select! {
        _ = cancellation.cancelled() => {
            return Err(LlmError::new(LlmErrorKind::Cancelled, "Request cancelled"));
        }
        result = connect => {
            result
                .map_err(|_| LlmError::unavailable("Timed out connecting to local inference"))?
                .map_err(|error| LlmError::unavailable(format!("Local inference is unavailable: {error}")))?
        }
    };
    stream.set_nodelay(true).map_err(LlmError::from)?;

    let body = body.unwrap_or(&[]);
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json, text/event-stream\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        endpoint.authority(),
        body.len()
    );
    if let Some(key) = api_key {
        validate_header_safe_value("API key", key, 4096)?;
        head.push_str("Authorization: Bearer ");
        head.push_str(key);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    let write_request = async {
        stream.write_all(head.as_bytes()).await?;
        if !body.is_empty() {
            stream.write_all(body).await?;
        }
        stream.flush().await
    };
    tokio::select! {
        _ = cancellation.cancelled() => {
            return Err(LlmError::new(LlmErrorKind::Cancelled, "Request cancelled"));
        }
        result = timeout(idle_timeout, write_request) => {
            result
                .map_err(|_| LlmError::unavailable("Timed out writing to local inference"))?
                .map_err(LlmError::from)?;
        }
    }

    let mut received = Vec::with_capacity(4096);
    let header_end = loop {
        if let Some(index) = find_bytes(&received, b"\r\n\r\n") {
            break index + 4;
        }
        if received.len() >= MAX_HTTP_HEAD_BYTES {
            return Err(LlmError::protocol(
                "Local inference HTTP headers are too large",
            ));
        }
        let mut buffer = [0u8; 4096];
        let count = tokio::select! {
            _ = cancellation.cancelled() => {
                return Err(LlmError::new(LlmErrorKind::Cancelled, "Request cancelled"));
            }
            result = timeout(idle_timeout, stream.read(&mut buffer)) => {
                result
                    .map_err(|_| LlmError::unavailable("Timed out waiting for local inference"))?
                    .map_err(LlmError::from)?
            }
        };
        if count == 0 {
            return Err(LlmError::protocol(
                "Local inference closed before sending HTTP headers",
            ));
        }
        received.extend_from_slice(&buffer[..count]);
    };

    let headers = parse_http_head(&received[..header_end])?;
    let initial_body = received[header_end..].to_vec();
    let framing = if headers
        .headers
        .get("transfer-encoding")
        .is_some_and(|value| {
            value
                .split(',')
                .any(|encoding| encoding.trim().eq_ignore_ascii_case("chunked"))
        }) {
        BodyFraming::Chunked(ChunkedDecoder::new(initial_body))
    } else if let Some(length) = headers.headers.get("content-length") {
        let remaining = length
            .parse::<usize>()
            .map_err(|_| LlmError::protocol("Invalid Content-Length from local inference"))?;
        BodyFraming::ContentLength {
            pending: initial_body,
            remaining,
        }
    } else {
        BodyFraming::UntilEof {
            pending: initial_body,
        }
    };

    Ok(HttpResponse {
        status: headers.status,
        body: HttpBody {
            stream,
            framing,
            idle_timeout,
        },
    })
}

struct ParsedHttpHead {
    status: u16,
    headers: HashMap<String, String>,
}

fn parse_http_head(bytes: &[u8]) -> Result<ParsedHttpHead, LlmError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LlmError::protocol("Local inference sent non-UTF-8 HTTP headers"))?;
    let mut lines = text.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| LlmError::protocol("Missing HTTP status line"))?;
    let mut status_parts = status_line.split_whitespace();
    let version = status_parts.next().unwrap_or_default();
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(LlmError::protocol(
            "Unsupported local inference HTTP version",
        ));
    }
    let status = status_parts
        .next()
        .ok_or_else(|| LlmError::protocol("Missing HTTP status code"))?
        .parse::<u16>()
        .map_err(|_| LlmError::protocol("Invalid HTTP status code"))?;
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| LlmError::protocol("Malformed local inference HTTP header"))?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    Ok(ParsedHttpHead { status, headers })
}

struct HttpBody {
    stream: TcpStream,
    framing: BodyFraming,
    idle_timeout: Duration,
}

enum BodyFraming {
    ContentLength { pending: Vec<u8>, remaining: usize },
    Chunked(ChunkedDecoder),
    UntilEof { pending: Vec<u8> },
    Finished,
}

impl HttpBody {
    async fn next_data(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<Option<Vec<u8>>, LlmError> {
        loop {
            match &mut self.framing {
                BodyFraming::ContentLength { pending, remaining } => {
                    if *remaining == 0 {
                        self.framing = BodyFraming::Finished;
                        return Ok(None);
                    }
                    if !pending.is_empty() {
                        let count = pending.len().min(*remaining);
                        let data: Vec<u8> = pending.drain(..count).collect();
                        *remaining -= count;
                        return Ok(Some(data));
                    }
                }
                BodyFraming::Chunked(decoder) => {
                    if let Some(data) = decoder.next_decoded()? {
                        return Ok(Some(data));
                    }
                    if decoder.is_finished() {
                        self.framing = BodyFraming::Finished;
                        return Ok(None);
                    }
                }
                BodyFraming::UntilEof { pending } => {
                    if !pending.is_empty() {
                        return Ok(Some(std::mem::take(pending)));
                    }
                }
                BodyFraming::Finished => return Ok(None),
            }

            let mut buffer = vec![0u8; 16 * 1024];
            let count = tokio::select! {
                _ = cancellation.cancelled() => {
                    return Err(LlmError::new(LlmErrorKind::Cancelled, "Generation cancelled"));
                }
                result = timeout(self.idle_timeout, self.stream.read(&mut buffer)) => {
                    result
                        .map_err(|_| LlmError::unavailable("Local inference stream timed out"))?
                        .map_err(LlmError::from)?
                }
            };
            if count == 0 {
                return match &self.framing {
                    BodyFraming::UntilEof { .. } => {
                        self.framing = BodyFraming::Finished;
                        Ok(None)
                    }
                    BodyFraming::Finished => Ok(None),
                    _ => Err(LlmError::protocol(
                        "Local inference closed before completing the HTTP body",
                    )),
                };
            }
            buffer.truncate(count);
            match &mut self.framing {
                BodyFraming::ContentLength { pending, .. } | BodyFraming::UntilEof { pending } => {
                    pending.extend_from_slice(&buffer)
                }
                BodyFraming::Chunked(decoder) => decoder.push(&buffer)?,
                BodyFraming::Finished => return Ok(None),
            }
        }
    }

    async fn read_all(
        &mut self,
        cancellation: &CancellationToken,
        limit: usize,
    ) -> Result<Vec<u8>, LlmError> {
        let mut output = Vec::new();
        while let Some(chunk) = self.next_data(cancellation).await? {
            if output.len().saturating_add(chunk.len()) > limit {
                return Err(LlmError::protocol("Local inference response is too large"));
            }
            output.extend_from_slice(&chunk);
        }
        Ok(output)
    }
}

struct ChunkedDecoder {
    input: Vec<u8>,
    state: ChunkState,
}

enum ChunkState {
    Size,
    Data(usize),
    DataCrlf,
    Trailers,
    Finished,
}

impl ChunkedDecoder {
    fn new(input: Vec<u8>) -> Self {
        Self {
            input,
            state: ChunkState::Size,
        }
    }

    fn push(&mut self, bytes: &[u8]) -> Result<(), LlmError> {
        if self.input.len().saturating_add(bytes.len()) > MAX_CHUNK_SIZE + MAX_HTTP_HEAD_BYTES {
            return Err(LlmError::protocol(
                "Chunked inference response buffer is too large",
            ));
        }
        self.input.extend_from_slice(bytes);
        Ok(())
    }

    fn is_finished(&self) -> bool {
        matches!(self.state, ChunkState::Finished)
    }

    fn next_decoded(&mut self) -> Result<Option<Vec<u8>>, LlmError> {
        loop {
            match self.state {
                ChunkState::Size => {
                    let Some(end) = find_bytes(&self.input, b"\r\n") else {
                        if self.input.len() > MAX_HTTP_HEAD_BYTES {
                            return Err(LlmError::protocol("HTTP chunk-size line is too large"));
                        }
                        return Ok(None);
                    };
                    let line = std::str::from_utf8(&self.input[..end])
                        .map_err(|_| LlmError::protocol("Invalid HTTP chunk-size line"))?;
                    let size_text = line.split(';').next().unwrap_or_default().trim();
                    let size = usize::from_str_radix(size_text, 16)
                        .map_err(|_| LlmError::protocol("Invalid HTTP chunk size"))?;
                    if size > MAX_CHUNK_SIZE {
                        return Err(LlmError::protocol("HTTP chunk is too large"));
                    }
                    self.input.drain(..end + 2);
                    self.state = if size == 0 {
                        ChunkState::Trailers
                    } else {
                        ChunkState::Data(size)
                    };
                }
                ChunkState::Data(remaining) => {
                    if self.input.is_empty() {
                        return Ok(None);
                    }
                    let count = remaining.min(self.input.len());
                    let output: Vec<u8> = self.input.drain(..count).collect();
                    self.state = if count == remaining {
                        ChunkState::DataCrlf
                    } else {
                        ChunkState::Data(remaining - count)
                    };
                    return Ok(Some(output));
                }
                ChunkState::DataCrlf => {
                    if self.input.len() < 2 {
                        return Ok(None);
                    }
                    if &self.input[..2] != b"\r\n" {
                        return Err(LlmError::protocol("Malformed HTTP chunk terminator"));
                    }
                    self.input.drain(..2);
                    self.state = ChunkState::Size;
                }
                ChunkState::Trailers => {
                    if self.input.starts_with(b"\r\n") {
                        self.input.drain(..2);
                        self.state = ChunkState::Finished;
                    } else if let Some(end) = find_bytes(&self.input, b"\r\n\r\n") {
                        self.input.drain(..end + 4);
                        self.state = ChunkState::Finished;
                    } else {
                        if self.input.len() > MAX_HTTP_HEAD_BYTES {
                            return Err(LlmError::protocol("HTTP trailers are too large"));
                        }
                        return Ok(None);
                    }
                }
                ChunkState::Finished => return Ok(None),
            }
        }
    }
}

#[derive(Default)]
struct SseDecoder {
    input: Vec<u8>,
    data_lines: Vec<Vec<u8>>,
}

impl SseDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, LlmError> {
        self.input.extend_from_slice(bytes);
        if self.input.len() > MAX_JSON_BODY_BYTES {
            return Err(LlmError::protocol("Inference event is too large"));
        }
        let mut events = Vec::new();
        while let Some(end) = self.input.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.input.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.is_empty() {
                if !self.data_lines.is_empty() {
                    events.push(join_data_lines(std::mem::take(&mut self.data_lines)));
                }
            } else if line.starts_with(b"data:") {
                let mut data = line[5..].to_vec();
                if data.first() == Some(&b' ') {
                    data.remove(0);
                }
                self.data_lines.push(data);
            }
        }
        Ok(events)
    }

    fn finish(&mut self) -> Result<Vec<Vec<u8>>, LlmError> {
        let mut events = self.push(b"\n\n")?;
        if !self.input.is_empty() || !self.data_lines.is_empty() {
            return Err(LlmError::protocol("Incomplete server-sent event"));
        }
        events.retain(|event| !event.is_empty());
        Ok(events)
    }
}

fn join_data_lines(lines: Vec<Vec<u8>>) -> Vec<u8> {
    let capacity = lines.iter().map(Vec::len).sum::<usize>() + lines.len().saturating_sub(1);
    let mut output = Vec::with_capacity(capacity);
    for (index, line) in lines.into_iter().enumerate() {
        if index > 0 {
            output.push(b'\n');
        }
        output.extend(line);
    }
    output
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn http_status_error(status: u16, body: &[u8]) -> LlmError {
    let message = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("Local inference returned HTTP {status}"));
    let kind = match status {
        401 | 403 => LlmErrorKind::Authentication,
        400 | 404 | 405 | 422 => LlmErrorKind::Protocol,
        429 | 500..=599 => LlmErrorKind::Server,
        _ => LlmErrorKind::Server,
    };
    LlmError::new(kind, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn endpoint_parser_is_strictly_local() {
        assert!(LoopbackEndpoint::parse("http://127.0.0.1:8080").is_ok());
        assert!(LoopbackEndpoint::parse("http://localhost:8080/").is_ok());
        assert!(LoopbackEndpoint::parse("http://[::1]:8080").is_ok());
        assert!(LoopbackEndpoint::parse("https://127.0.0.1:8080").is_err());
        assert!(LoopbackEndpoint::parse("http://192.168.1.10:8080").is_err());
        assert!(LoopbackEndpoint::parse("http://example.com:8080").is_err());
        assert!(LoopbackEndpoint::parse("http://127.0.0.1:8080/v1").is_err());
    }

    #[test]
    fn local_defaults_do_not_trust_an_implicit_loopback_process() {
        let config = RuntimeConfig::local_defaults(None);
        assert!(config.loopback_candidates.is_empty());
        assert_eq!(config.parallel_sequences, 1);
        assert_eq!(config.context_tokens, 8192);
        assert_eq!(config.generation_idle_timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn verifies_and_streams_chunked_llama_events() {
        let (endpoint, server) = spawn_mock_llama_server(false).await;
        let provider = LoopbackLlamaServer::new(endpoint, Some("test-secret".to_string()), None)
            .expect("provider");

        let info = provider.verify().await.expect("verified provider");
        assert_eq!(info.model_ids, vec!["mock-gguf"]);

        let mut stream = provider
            .start_stream(GenerationRequest::terminal_assistant("say hello"))
            .expect("stream");
        let mut output = String::new();
        let mut saw_finished = false;
        while let Some(event) = stream.recv().await {
            match event.expect("valid event") {
                StreamEvent::Delta { text } => output.push_str(&text),
                StreamEvent::Finished { metrics } => {
                    assert_eq!(metrics.completion_tokens, Some(2));
                    saw_finished = true;
                }
                StreamEvent::Started { .. } => {}
            }
        }
        assert_eq!(output, "hello ✓");
        assert!(saw_finished);
        server.await.expect("mock server");
    }

    #[tokio::test]
    async fn cancellation_interrupts_a_pending_stream() {
        let (endpoint, server) = spawn_mock_llama_server(true).await;
        let provider = LoopbackLlamaServer::new(endpoint, None, Some("mock-gguf".to_string()))
            .expect("provider");
        let mut stream = provider
            .start_stream(GenerationRequest::terminal_assistant("wait"))
            .expect("stream");
        sleep(Duration::from_millis(25)).await;
        stream.cancel();
        let event = timeout(Duration::from_secs(1), stream.recv())
            .await
            .expect("cancellation timeout")
            .expect("cancellation result")
            .expect_err("cancelled error");
        assert_eq!(event.kind, LlmErrorKind::Cancelled);
        server.await.expect("mock server");
    }

    async fn spawn_mock_llama_server(
        stall_chat: bool,
    ) -> (LoopbackEndpoint, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind mock");
        let endpoint =
            LoopbackEndpoint::new(listener.local_addr().expect("mock address")).expect("loopback");
        let request_count = if stall_chat { 1 } else { 3 };
        let server = tokio::spawn(async move {
            for _ in 0..request_count {
                let (mut socket, _) = listener.accept().await.expect("accept mock");
                let (path, request) = read_mock_request(&mut socket).await;
                if path == "/health" {
                    write_mock_json(&mut socket, r#"{"status":"ok"}"#).await;
                } else if path == "/v1/models" {
                    assert!(request.contains("Authorization: Bearer test-secret"));
                    write_mock_json(&mut socket, r#"{"data":[{"id":"mock-gguf"}]}"#).await;
                } else if path == "/v1/chat/completions" && stall_chat {
                    socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
                        )
                        .await
                        .expect("write stalled headers");
                    sleep(Duration::from_millis(100)).await;
                } else if path == "/v1/chat/completions" {
                    assert!(request.contains("\"stream\":true"));
                    let events = concat!(
                        "data: {\"id\":\"mock-request\",\"model\":\"mock-gguf\",\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\n",
                        "data: {\"choices\":[{\"delta\":{\"content\":\"\\u2713\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2,\"total_tokens\":5}}\n\n",
                        "data: [DONE]\n\n"
                    );
                    write_mock_chunked(&mut socket, events.as_bytes()).await;
                } else {
                    panic!("unexpected mock path {path}");
                }
            }
        });
        (endpoint, server)
    }

    async fn read_mock_request(socket: &mut TcpStream) -> (String, String) {
        let mut request = Vec::new();
        let header_end = loop {
            if let Some(end) = find_bytes(&request, b"\r\n\r\n") {
                break end + 4;
            }
            let mut buffer = [0u8; 4096];
            let count = socket.read(&mut buffer).await.expect("read mock request");
            assert!(count > 0);
            request.extend_from_slice(&buffer[..count]);
        };
        let head = String::from_utf8_lossy(&request[..header_end]);
        let content_length = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let mut buffer = [0u8; 4096];
            let count = socket.read(&mut buffer).await.expect("read mock body");
            assert!(count > 0);
            request.extend_from_slice(&buffer[..count]);
        }
        let text = String::from_utf8_lossy(&request).to_string();
        let path = text
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or_default()
            .to_string();
        (path, text)
    }

    async fn write_mock_json(socket: &mut TcpStream, body: &str) {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write mock JSON");
    }

    async fn write_mock_chunked(socket: &mut TcpStream, body: &[u8]) {
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
            )
            .await
            .expect("write mock headers");
        for chunk in body.chunks(17) {
            let head = format!("{:X}\r\n", chunk.len());
            socket.write_all(head.as_bytes()).await.expect("chunk head");
            socket.write_all(chunk).await.expect("chunk data");
            socket.write_all(b"\r\n").await.expect("chunk end");
        }
        socket.write_all(b"0\r\n\r\n").await.expect("last chunk");
    }
}
