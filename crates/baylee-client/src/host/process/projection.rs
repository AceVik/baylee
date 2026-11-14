//! One bounded, cancellable local replay job through the shared process owner.
use super::{
    supervisor::{self, Target},
    wire::{Bootstrap, Contract, Initial, recorded_preset},
};
use baylee_client_core::replay::{Header, Tape, Verify};
use std::{
    process::Command,
    sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
    time::Duration,
};

/// An owned local record and the perspective its holder asked to replay.
pub struct Projection {
    /// JSON Lines, bounded by the existing local-record limit.
    pub record: Vec<u8>,
    /// Display names, in seat order.
    pub names: Vec<String>,
    /// The requested perspective and tape metadata; its seat is validated.
    pub header: Header,
    /// Whether to stop at a deterministic mismatch or continue with annotations.
    pub verify: Verify,
}

impl Projection {
    /// Runs on a host worker thread, never the renderer. Cancellation kills and
    /// reaps the child before returning, including a stuck rules operation.
    ///
    /// # Errors
    /// Invalid record, incompatible worker, resource limit or cancellation.
    pub fn project(self, progress: &AtomicU8, cancel: &AtomicBool) -> Result<Tape, String> {
        let command = Command::new(
            std::env::current_exe().map_err(|_| "the client executable is unavailable")?,
        );
        self.with_command(command, progress, cancel)
    }

    /// Uses an explicitly trusted local client executable, for embedders and
    /// subprocess integration tests. No path is taken from replay contents.
    ///
    /// # Errors
    /// The same failures as [`Self::project`], or a command with extra arguments.
    pub fn with_command(
        self,
        command: Command,
        progress: &AtomicU8,
        cancel: &AtomicBool,
    ) -> Result<Tape, String> {
        if command.get_args().next().is_some()
            || self.record.len() > baylee_client_core::bugreport::MAX_INFLATED_BYTES
        {
            return Err("the local replay request is invalid".into());
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("the local replay was cancelled".into());
        }
        let preset = recorded_preset(&self.record)?;
        if usize::from(self.header.seat) >= preset.seats.len()
            || self.names.len() != preset.seats.len()
        {
            return Err("the local replay seat is invalid".into());
        }
        let initial = Bootstrap {
            contract: Contract::new(preset.house_rules.loop_policy),
            seat: baylee_core::ids::PlayerId::new(self.header.seat),
            names: self.names,
            persist: false,
            initial: Initial::Replay {
                record: self.record,
                header: serde_json::to_vec(&self.header)
                    .map_err(|_| "the replay header could not be encoded")?,
                verify_note: self.verify == Verify::Note,
            },
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "the local replay supervisor could not start")?;
        let mut lines = Vec::new();
        let pid = AtomicU32::new(0);
        progress.store(0, Ordering::Relaxed);
        runtime.block_on(async {
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let task = supervisor::run(
                command,
                initial,
                Target::Replay {
                    progress,
                    bytes: &mut lines,
                },
                stopped,
                &pid,
                baylee_worker::Limits::default(),
            );
            tokio::pin!(task);
            tokio::select! {
                biased;
                () = cancelled(cancel) => {
                    drop(stop);
                    task.await?;
                    Err("the local replay was cancelled".to_string())
                },
                result = &mut task => result,
            }
        })?;
        if cancel.load(Ordering::Relaxed) {
            return Err("the local replay was cancelled".into());
        }
        Tape::read_lines(&lines).map_err(|why| why.to_string())
    }
}

async fn cancelled(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
