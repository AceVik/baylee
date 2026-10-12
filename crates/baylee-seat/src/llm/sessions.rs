//! Optional endpoint-owned conversations. No model calls are retried here.
use super::lock;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(2);

struct State {
    id: Option<String>,
}

/// One remote conversation, owned by exactly one game/seat pair.
pub(super) struct Session {
    agent: ureq::Agent,
    base: String,
    headers: Vec<(&'static str, String)>,
    label: String,
    closed: AtomicBool,
    heartbeat_secs: AtomicU64,
    state: Mutex<State>,
    turn: Mutex<()>,
    changed: Arc<tokio::sync::Notify>,
}
impl Session {
    pub fn new(
        agent: ureq::Agent,
        base: String,
        headers: Vec<(&'static str, String)>,
        seat: u8,
    ) -> Arc<Self> {
        let session = Arc::new(Self {
            agent,
            base,
            headers,
            // No player name, room name, profile, or actual game identifier.
            label: format!("game {} / seat {seat}", uuid::Uuid::now_v7()),
            closed: AtomicBool::new(false),
            heartbeat_secs: AtomicU64::new(300),
            changed: Arc::default(),
            state: Mutex::new(State { id: None }),
            turn: Mutex::new(()),
        });
        let weak = Arc::downgrade(&session);
        tokio::spawn(async move {
            loop {
                let Some(session) = weak.upgrade() else { break };
                if session.closed.load(Ordering::Acquire) {
                    break;
                }
                let interval = Duration::from_secs(session.heartbeat_secs.load(Ordering::Relaxed));
                let changed = session.changed.clone();
                drop(session);
                tokio::select! {
                    () = changed.notified() => continue,
                    () = tokio::time::sleep(interval) => {}
                }
                let Some(session) = weak.upgrade() else { break };
                let _ = tokio::task::spawn_blocking(move || session.touch()).await;
            }
        });
        session
    }
    // Called in the model's blocking worker. It remains owned if the async
    // caller stops waiting while a successful creation is arriving.
    pub fn id(&self, timeout: Duration) -> Result<String, String> {
        let mut state = lock(&self.state);
        if self.closed.load(Ordering::Acquire) {
            return Err("game session is closed".into());
        }
        if state.id.is_none() {
            let (status, value) =
                self.request("", Some(&json!({"label":self.label})), timeout.min(TIMEOUT))?;
            if status != 200 && status != 201 {
                return Err(format!("session creation refused ({status})"));
            }
            let id = value
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| valid_id(id))
                .ok_or("session endpoint returned an invalid identifier")?
                .to_owned();
            let idle = value
                .get("idle_timeout_secs")
                .and_then(Value::as_u64)
                .unwrap_or(7200);
            self.heartbeat_secs
                .store((idle / 3).clamp(1, 300), Ordering::Relaxed);
            state.id = Some(id);
            self.changed.notify_one();
        }
        if self.closed.load(Ordering::Acquire) {
            return Err("game session closed during creation".into());
        }
        Ok(state.id.as_ref().expect("created above").clone())
    }
    // The blocking HTTP worker outlives a cancelled decision future. Never send
    // another turn into the same conversation until that worker has returned.
    pub fn turn(&self) -> Result<MutexGuard<'_, ()>, String> {
        self.turn
            .try_lock()
            .map_err(|_| "a previous session turn is still finishing".into())
    }
    pub fn forget_expired(&self, expired: &str) {
        let mut state = lock(&self.state);
        if state.id.as_deref() == Some(expired) {
            state.id = None;
        }
    }
    fn touch(&self) {
        let mut state = lock(&self.state);
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        let Some(id) = state.id.as_deref() else {
            return;
        };
        if matches!(
            self.request(&format!("/{id}/touch"), Some(&json!({})), TIMEOUT),
            Ok((404 | 410, _))
        ) {
            state.id = None;
        }
    }
    /// Mark closed synchronously, then delete off the async executor. A lost
    /// response or a hard kill is covered by the server's inactivity expiry.
    pub fn close(self: &Arc<Self>) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        self.changed.notify_one();
        let session = self.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn_blocking(move || {
                let id = lock(&session.state).id.take();
                if let Some(id) = id {
                    let _ = session.request(&format!("/{id}"), None, TIMEOUT);
                }
            });
        }
    }
    fn request(
        &self,
        suffix: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<(u16, Value), String> {
        let url = format!("{}/sessions{suffix}", self.base);
        let mut response = if let Some(body) = body {
            let mut request = self.agent.post(&url);
            for (name, value) in &self.headers {
                request = request.header(*name, value);
            }
            request
                .config()
                .timeout_global(Some(timeout))
                .build()
                .send_json(body)
        } else {
            let mut request = self.agent.delete(&url);
            for (name, value) in &self.headers {
                request = request.header(*name, value);
            }
            request
                .config()
                .timeout_global(Some(timeout))
                .build()
                .call()
        }
        .map_err(|_| "session endpoint is unavailable".to_string())?;
        let status = response.status().as_u16();
        // Ignore failure bodies: some proxies reflect credentials in them.
        if !(200..300).contains(&status) || body.is_none() {
            return Ok((status, Value::Null));
        }
        let value = response
            .body_mut()
            .with_config()
            .limit(4096)
            .read_json()
            .map_err(|_| "invalid session endpoint response".to_string())?;
        Ok((status, value))
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

#[cfg(test)]
mod tests;
