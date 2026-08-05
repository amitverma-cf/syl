use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex as AsyncMutex};

use crate::capability::{check_requirements, HOST_CAPABILITIES};
use crate::manifest::ExtensionManifest;
use crate::protocol::RpcMessage;

#[derive(Debug, thiserror::Error)]
pub enum ExtensionProcessError {
    #[error("extension {0:?} requires an unsupported capability: {1}")]
    UnsupportedRequirement(String, #[source] crate::capability::UnsupportedRequirement),
    #[error("failed to spawn extension backend: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("failed to write to extension process: {0}")]
    Write(#[source] std::io::Error),
    #[error("extension process crashed or exited unexpectedly")]
    Crashed,
    #[error(
        "extension {extension:?}'s manifest declares capability {capability:?}, but its \
         initialize response did not confirm it"
    )]
    CapabilityMismatch {
        extension: String,
        capability: String,
    },
    #[error("extension does not provide capability {0:?}")]
    CapabilityNotProvided(String),
    #[error("extension returned an error: {0}")]
    Extension(String),
    #[error("malformed response from extension: {0}")]
    Malformed(String),
    #[error("extension did not respond within {0:?}")]
    Timeout(std::time::Duration),
    #[error("too many in-flight requests to this extension ({MAX_PENDING_REQUESTS} max)")]
    TooManyPendingRequests,
    #[error("extension {0:?} has no backend process to spawn (it's a UI-only extension)")]
    NoBackend(String),
}

/// How long a single request (`initialize`, `countTokens`, or one `generate`
/// call) is allowed to take before it's treated as hung rather than merely
/// slow. A hang is a different failure mode than a crash — the reader task
/// never sees stdout close, so nothing else would ever unblock the caller.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// A defensive cap on concurrent in-flight requests to one extension
/// process. Chat is turn-based in practice (one `generate` at a time per
/// loaded model), so this should never be reached in normal use — it exists
/// to fail clearly if a caller bug ever fires off requests faster than the
/// extension can drain them, rather than growing `pending` unbounded.
const MAX_PENDING_REQUESTS: usize = 64;

/// One event routed to an in-flight request's channel, translated from the
/// raw `RpcMessage` the reader task parses off the extension's stdout.
enum WorkerEvent {
    Piece(String),
    Done(Value),
    Error(String),
    Crashed,
}

/// A running extension backend process. Spawns the manifest's declared
/// command, then routes JSON-RPC-shaped messages between host requests and
/// the extension's responses/notifications. If the child process exits
/// unexpectedly (crash, panic across an FFI boundary inside it, whatever),
/// every in-flight request gets a clean `Crashed` error instead of the host
/// process itself going down — this is the actual point of the whole
/// extension-ecosystem pass.
pub struct ExtensionProcess {
    manifest: ExtensionManifest,
    child: AsyncMutex<Child>,
    stdin: AsyncMutex<ChildStdin>,
    next_id: AtomicU64,
    pending: Arc<std::sync::Mutex<HashMap<u64, mpsc::UnboundedSender<WorkerEvent>>>>,
    dead: Arc<AtomicBool>,
    request_timeout: std::time::Duration,
    // Kept alive for the process's lifetime: a Windows Job Object with
    // `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` set, so the OS kills this child
    // automatically if the host process itself dies abnormally (crash,
    // `taskkill /F`) instead of exiting cleanly — ordinary parent/child
    // semantics do not guarantee that on their own.
    #[cfg(windows)]
    _job: Option<win32job::Job>,
}

impl ExtensionProcess {
    pub async fn spawn(manifest: ExtensionManifest) -> Result<Self, ExtensionProcessError> {
        Self::spawn_with_timeout(manifest, REQUEST_TIMEOUT).await
    }

    /// Same as [`Self::spawn`], but with an explicit per-request timeout
    /// instead of the default [`REQUEST_TIMEOUT`] — mainly for tests, which
    /// shouldn't have to wait the full production timeout to prove a hang
    /// is detected.
    pub async fn spawn_with_timeout(
        manifest: ExtensionManifest,
        request_timeout: std::time::Duration,
    ) -> Result<Self, ExtensionProcessError> {
        check_requirements(&manifest.requires)
            .map_err(|e| ExtensionProcessError::UnsupportedRequirement(manifest.id.clone(), e))?;

        let backend = manifest
            .backend
            .clone()
            .ok_or_else(|| ExtensionProcessError::NoBackend(manifest.id.clone()))?;

        let mut command = Command::new(&backend.command);
        command
            .args(&backend.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = command.spawn().map_err(ExtensionProcessError::Spawn)?;
        let stdin = child.stdin.take().expect("configured with piped stdin");
        let stdout = child.stdout.take().expect("configured with piped stdout");
        let stderr = child.stderr.take().expect("configured with piped stderr");

        let pending: Arc<std::sync::Mutex<HashMap<u64, mpsc::UnboundedSender<WorkerEvent>>>> =
            Arc::new(std::sync::Mutex::new(HashMap::new()));
        let dead = Arc::new(AtomicBool::new(false));

        spawn_stdout_reader(stdout, pending.clone(), dead.clone());
        spawn_stderr_logger(stderr, manifest.id.clone());

        #[cfg(windows)]
        let job = assign_kill_on_close_job(&child, &manifest.id);

        let process = Self {
            manifest,
            child: AsyncMutex::new(child),
            stdin: AsyncMutex::new(stdin),
            next_id: AtomicU64::new(1),
            pending,
            dead,
            request_timeout,
            #[cfg(windows)]
            _job: job,
        };

        if let Err(err) = process.initialize().await {
            // The child is already running at this point — a failed
            // handshake (mismatch, hang, crash) must not leak it. `spawn`
            // returning `Err` should mean "no process is running", the same
            // contract as if `Command::spawn` itself had failed.
            process.kill().await;
            return Err(err);
        }
        Ok(process)
    }

    async fn initialize(&self) -> Result<(), ExtensionProcessError> {
        let response = self
            .request(
                "initialize",
                json!({ "hostCapabilities": HOST_CAPABILITIES }),
            )
            .await?;
        let provided: Vec<String> = response
            .get("provides")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        for expected in &self.manifest.provides {
            if !provided.contains(expected) {
                return Err(ExtensionProcessError::CapabilityMismatch {
                    extension: self.manifest.id.clone(),
                    capability: expected.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn is_alive(&self) -> bool {
        !self.dead.load(Ordering::SeqCst)
    }

    pub fn provides(&self, capability: &str) -> bool {
        self.manifest.provides.iter().any(|c| c == capability)
    }

    /// Generic, capability-gated one-shot call — the entry point for every
    /// non-chat extension (image generation, embeddings, ASR, TTS): each
    /// gets its own capability id and method name instead of a bespoke Rust
    /// method per capability. `generate()` stays separate since it's the
    /// only streaming/notification-based call.
    pub async fn call(
        &self,
        capability: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, ExtensionProcessError> {
        if !self.provides(capability) {
            return Err(ExtensionProcessError::CapabilityNotProvided(
                capability.to_string(),
            ));
        }
        self.request(method, params).await
    }

    /// Sends a request and waits for its single JSON response — used for
    /// non-streaming calls (`initialize`, `countTokens`).
    async fn request(&self, method: &str, params: Value) -> Result<Value, ExtensionProcessError> {
        if self.dead.load(Ordering::SeqCst) {
            return Err(ExtensionProcessError::Crashed);
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut pending = self.pending.lock().unwrap();
            if pending.len() >= MAX_PENDING_REQUESTS {
                return Err(ExtensionProcessError::TooManyPendingRequests);
            }
            pending.insert(id, tx);
        }

        self.send(RpcMessage::request(id, method, params)).await?;

        let event = tokio::time::timeout(self.request_timeout, rx.recv()).await;
        self.pending.lock().unwrap().remove(&id);
        match event {
            Ok(Some(WorkerEvent::Done(value))) => Ok(value),
            Ok(Some(WorkerEvent::Error(message))) => Err(ExtensionProcessError::Extension(message)),
            Ok(Some(WorkerEvent::Crashed)) | Ok(None) => Err(ExtensionProcessError::Crashed),
            Ok(Some(WorkerEvent::Piece(_))) => Err(ExtensionProcessError::Malformed(
                "unexpected streaming piece for a non-streaming request".to_string(),
            )),
            Err(_elapsed) => Err(ExtensionProcessError::Timeout(self.request_timeout)),
        }
    }

    async fn send(&self, message: RpcMessage) -> Result<(), ExtensionProcessError> {
        let mut line = message
            .to_line()
            .map_err(|e| ExtensionProcessError::Malformed(e.to_string()))?;
        line.push('\n');
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(ExtensionProcessError::Write)?;
        stdin.flush().await.map_err(ExtensionProcessError::Write)
    }

    /// Calls the `inference.chat/v1` capability's `inference/countTokens`
    /// method.
    pub async fn count_tokens(&self, text: &str) -> Result<usize, ExtensionProcessError> {
        if !self.provides("inference.chat/v1") {
            return Err(ExtensionProcessError::CapabilityNotProvided(
                "inference.chat/v1".to_string(),
            ));
        }
        let result = self
            .request("inference/countTokens", json!({ "text": text }))
            .await?;
        result
            .get("count")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .ok_or_else(|| {
                ExtensionProcessError::Malformed("countTokens response missing count".to_string())
            })
    }

    /// Calls the `inference.chat/v1` capability's `inference/generate`
    /// method, streaming `inference/piece` notifications to `on_piece` as
    /// they arrive — same `impl FnMut(&str)` shape the pre-extension
    /// in-process `LlamaEngine::generate` callback used, so callers
    /// (`native_engines::tool_loop::generate_with_tools`, `commands.rs`) don't
    /// need to change shape, only which concrete type they call into.
    pub async fn generate(
        &self,
        prompt: &str,
        max_tokens: i32,
        mut on_piece: impl FnMut(&str),
    ) -> Result<String, ExtensionProcessError> {
        if !self.provides("inference.chat/v1") {
            return Err(ExtensionProcessError::CapabilityNotProvided(
                "inference.chat/v1".to_string(),
            ));
        }
        if self.dead.load(Ordering::SeqCst) {
            return Err(ExtensionProcessError::Crashed);
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut pending = self.pending.lock().unwrap();
            if pending.len() >= MAX_PENDING_REQUESTS {
                return Err(ExtensionProcessError::TooManyPendingRequests);
            }
            pending.insert(id, tx);
        }

        self.send(RpcMessage::request(
            id,
            "inference/generate",
            json!({ "prompt": prompt, "maxTokens": max_tokens }),
        ))
        .await?;

        loop {
            // The timeout resets on every received event (including each
            // streamed piece), not just once for the whole call — a model
            // that's actively streaming tokens shouldn't time out just
            // because the overall generation is long; only a period of
            // total silence should.
            match tokio::time::timeout(self.request_timeout, rx.recv()).await {
                Ok(Some(WorkerEvent::Piece(text))) => on_piece(&text),
                Ok(Some(WorkerEvent::Done(value))) => {
                    self.pending.lock().unwrap().remove(&id);
                    return value
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .ok_or_else(|| {
                            ExtensionProcessError::Malformed(
                                "generate response missing text".to_string(),
                            )
                        });
                }
                Ok(Some(WorkerEvent::Error(message))) => {
                    self.pending.lock().unwrap().remove(&id);
                    return Err(ExtensionProcessError::Extension(message));
                }
                Ok(Some(WorkerEvent::Crashed)) | Ok(None) => {
                    self.pending.lock().unwrap().remove(&id);
                    return Err(ExtensionProcessError::Crashed);
                }
                Err(_elapsed) => {
                    self.pending.lock().unwrap().remove(&id);
                    return Err(ExtensionProcessError::Timeout(self.request_timeout));
                }
            }
        }
    }

    /// Explicit shutdown, mirroring `McpConnectionHandle::disconnect`'s
    /// reasoning: don't wait for the last `Arc` clone to drop, kill the
    /// child process now.
    pub async fn kill(&self) {
        let _ = self.child.lock().await.kill().await;
    }
}

fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    pending: Arc<std::sync::Mutex<HashMap<u64, mpsc::UnboundedSender<WorkerEvent>>>>,
    dead: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            route_line(&line, &pending);
        }
        // stdout closed — the extension process exited (crashed, was
        // killed, or shut down cleanly). Mark it dead and unblock every
        // still-pending request with a crash error rather than leaving
        // callers waiting forever.
        dead.store(true, Ordering::SeqCst);
        let mut pending = pending.lock().unwrap();
        for (_, sender) in pending.drain() {
            let _ = sender.send(WorkerEvent::Crashed);
        }
    });
}

fn route_line(
    line: &str,
    pending: &std::sync::Mutex<HashMap<u64, mpsc::UnboundedSender<WorkerEvent>>>,
) {
    let Ok(message) = RpcMessage::from_line(line) else {
        tracing::warn!(%line, "extension sent a malformed protocol line");
        return;
    };

    if message.is_response() {
        let Some(id) = message.id else { return };
        let event = if let Some(error) = message.error {
            WorkerEvent::Error(error.message)
        } else {
            WorkerEvent::Done(message.result.unwrap_or(Value::Null))
        };
        if let Some(sender) = pending.lock().unwrap().get(&id) {
            let _ = sender.send(event);
        }
    } else if message.is_notification() && message.method.as_deref() == Some("inference/piece") {
        let Some(request_id) = message.params.get("requestId").and_then(|v| v.as_u64()) else {
            return;
        };
        let Some(text) = message.params.get("text").and_then(|v| v.as_str()) else {
            return;
        };
        if let Some(sender) = pending.lock().unwrap().get(&request_id) {
            let _ = sender.send(WorkerEvent::Piece(text.to_string()));
        }
    }
}

fn spawn_stderr_logger(stderr: tokio::process::ChildStderr, extension_id: String) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::warn!(extension = %extension_id, "{line}");
        }
    });
}

/// Puts `child` into a fresh Job Object configured with
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so Windows kills it automatically
/// when the returned `Job` handle closes — including when the host process
/// dies abnormally (crash, `taskkill /F`) rather than exiting cleanly, which
/// ordinary parent/child process semantics do not guarantee on their own.
/// Returns `None` (logging a warning) if the job couldn't be created or the
/// child couldn't be assigned to it; the extension still runs normally, it
/// just loses this specific orphan-prevention guarantee.
#[cfg(windows)]
fn assign_kill_on_close_job(child: &Child, extension_id: &str) -> Option<win32job::Job> {
    let job = match win32job::Job::create() {
        Ok(job) => job,
        Err(err) => {
            tracing::warn!(extension = %extension_id, "failed to create job object: {err}");
            return None;
        }
    };

    let mut info = match job.query_extended_limit_info() {
        Ok(info) => info,
        Err(err) => {
            tracing::warn!(extension = %extension_id, "failed to query job limit info: {err}");
            return None;
        }
    };
    info.limit_kill_on_job_close();
    if let Err(err) = job.set_extended_limit_info(&info) {
        tracing::warn!(extension = %extension_id, "failed to set job limit info: {err}");
        return None;
    }

    let Some(handle) = child.raw_handle() else {
        tracing::warn!(extension = %extension_id, "child has no raw handle to assign to job object");
        return None;
    };
    if let Err(err) = job.assign_process(handle as isize) {
        tracing::warn!(extension = %extension_id, "failed to assign process to job object: {err}");
        return None;
    }

    Some(job)
}
