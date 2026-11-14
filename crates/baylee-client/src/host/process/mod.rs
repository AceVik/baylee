//! Killable desktop local games over inherited private pipes. The renderer
//! never waits for rules work, I/O, process exit or replay reconstruction.
//! The child is this installed client executable in headless helper mode;
//! neither a server nor a player supplies an executable name.
use super::{DuelHost, GamePreset, HostMessage, PlayerId, ResumePoint};
use baylee_core::ids::CardIndex;
use std::{
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};
use tokio::sync::{mpsc, oneshot};
use wire::{Bootstrap, Contract, Initial, Input, Reply};

mod child;
pub mod projection;
mod supervisor;
mod wire;

/// Runs the private helper before opening any window, network service or
/// updater. Invalid invocations never fall through into the GUI.
#[must_use]
pub fn helper_if_asked() -> bool {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--local-engine")) {
        return false;
    }
    if args.next().is_some() || child::run().is_err() {
        // The owner reports a fixed technical interruption. No tape, cards,
        // private pipe contents or settings path enter stdout or diagnostics.
        std::process::exit(1);
    }
    true
}

/// Non-blocking renderer adapter for one owned desktop engine process.
pub struct ProcessHost {
    inputs: mpsc::Sender<Input>,
    replies: Mutex<mpsc::Receiver<Reply>>,
    stop: Option<oneshot::Sender<()>>,
    pid: Arc<AtomicU32>,
    seat: PlayerId,
    deck: Vec<CardIndex>,
    record: Vec<u8>,
    resume: Option<ResumePoint>,
    outstanding: usize,
    failed: bool,
    refusal: bool,
}

impl ProcessHost {
    /// Starts this installed executable's matching local engine.
    ///
    /// # Errors
    /// The local executable cannot be located or the worker cannot be scheduled.
    pub fn new(preset: &GamePreset, seat: PlayerId, names: &[&str]) -> Result<Self, String> {
        let command = Command::new(
            std::env::current_exe().map_err(|_| "the client executable is unavailable")?,
        );
        Self::launch(command, preset, seat, names)
    }

    /// Embeds a trusted, locally chosen client executable. This is a local
    /// application integration seam, never a path received over the network.
    /// The helper must prove its exact contract before publishing any state.
    ///
    /// # Errors
    /// Invalid setup or the supervisor thread cannot start. Child startup and
    /// rules failures arrive asynchronously as `HostMessage::Interrupted`.
    pub fn launch(
        command: Command,
        preset: &GamePreset,
        seat: PlayerId,
        names: &[&str],
    ) -> Result<Self, String> {
        let initial = Initial::New(
            serde_json::to_vec(preset).map_err(|_| "the local setup could not be encoded")?,
        );
        Self::start(
            command,
            preset,
            seat,
            names.iter().map(|n| (*n).into()).collect(),
            initial,
        )
    }

    /// Reconstructs a kept game in an owned process. The hash is checked there
    /// before any game state reaches the renderer.
    ///
    /// # Errors
    /// The record header is invalid or the supervisor cannot be scheduled.
    pub fn resume(
        command: Command,
        record: Vec<u8>,
        name: &str,
        seat: PlayerId,
        names: &[String],
        hash: &str,
    ) -> Result<Self, String> {
        if record.len() > baylee_client_core::bugreport::MAX_INFLATED_BYTES {
            return Err("the local record is too large".into());
        }
        let preset = wire::recorded_preset(&record)?;
        Self::start(
            command,
            &preset,
            seat,
            names.to_vec(),
            Initial::Resume {
                record,
                name: name.into(),
                hash: hash.into(),
            },
        )
    }

    fn start(
        command: Command,
        preset: &GamePreset,
        seat: PlayerId,
        names: Vec<String>,
        initial: Initial,
    ) -> Result<Self, String> {
        if command.get_args().next().is_some() {
            return Err("the local engine command must have no arguments".into());
        }
        if usize::from(seat.get()) >= preset.seats.len() || names.len() != preset.seats.len() {
            return Err("the local seat configuration is invalid".into());
        }
        let initial = Bootstrap {
            contract: Contract::new(preset.house_rules.loop_policy),
            initial,
            seat,
            names,
            persist: crate::settings::store_is_open(),
        };
        let (inputs, input) = mpsc::channel(8);
        let (output, replies) = mpsc::channel(1);
        let (stop, stopped) = oneshot::channel();
        let pid = Arc::new(AtomicU32::new(0));
        supervisor::spawn(command, initial, input, output, stopped, Arc::clone(&pid))?;
        Ok(Self {
            inputs,
            replies: Mutex::new(replies),
            stop: Some(stop),
            pid,
            seat,
            deck: super::own_deck(preset, seat),
            record: Vec::new(),
            resume: None,
            outstanding: 1,
            failed: false,
            refusal: false,
        })
    }

    /// The owned child PID for diagnostics; never used to send process signals.
    #[must_use]
    pub fn worker_id(&self) -> Option<u32> {
        let pid = self.pid.load(Ordering::Acquire);
        (pid != 0).then_some(pid)
    }

    fn send(&mut self, input: Input) {
        if self.failed {
            return;
        }
        if self.inputs.try_send(input).is_ok() {
            self.outstanding += 1;
        } else {
            self.failed = true;
            self.refusal = true;
            self.stop.take();
        }
    }
}

impl Drop for ProcessHost {
    fn drop(&mut self) {
        // Cancellation is non-blocking. The supervisor kills and reaps its
        // own Child; dropping the renderer never joins rules work.
        self.stop.take();
    }
}

impl DuelHost for ProcessHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        if std::mem::take(&mut self.refusal) {
            return vec![HostMessage::Interrupted];
        }
        if self.failed {
            return Vec::new();
        }
        match self
            .replies
            .get_mut()
            .expect("exclusive receiver")
            .try_recv()
        {
            Ok(reply) => {
                if self
                    .record
                    .len()
                    .checked_add(reply.record.len())
                    .is_none_or(|size| size > baylee_client_core::bugreport::MAX_INFLATED_BYTES)
                {
                    self.failed = true;
                    self.stop.take();
                    return vec![HostMessage::Interrupted];
                }
                self.record.extend(reply.record);
                if reply.done {
                    self.outstanding = self.outstanding.saturating_sub(1);
                    self.resume = reply.resume;
                }
                if reply
                    .messages
                    .iter()
                    .any(|m| matches!(m, HostMessage::Closed(_) | HostMessage::Interrupted))
                {
                    self.failed = true;
                    self.resume = None;
                }
                reply.messages
            }
            Err(mpsc::error::TryRecvError::Empty) => Vec::new(),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                self.failed = true;
                vec![HostMessage::Interrupted]
            }
        }
    }
    fn submit(&mut self, action: baylee_engine::choice::PlayerAction) {
        self.send(Input::Action(action));
    }
    fn seat_setting(&mut self, setting: baylee_view::SeatSetting) {
        self.send(Input::SeatSetting(setting));
    }
    fn ready(&mut self) {}
    fn seat(&self) -> PlayerId {
        self.seat
    }
    fn own_deck(&self) -> Vec<CardIndex> {
        self.deck.clone()
    }
    fn local_record(&self) -> Option<&[u8]> {
        Some(&self.record)
    }
    fn resume_point(&self) -> Option<ResumePoint> {
        (!self.failed && self.outstanding == 0)
            .then(|| self.resume.clone())
            .flatten()
    }
    fn busy(&self) -> bool {
        !self.failed && self.outstanding != 0
    }
}
