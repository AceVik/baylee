//! Starting asynchronous tape jobs, separate from replay presentation.
use super::{Job, Kept, ReplayHost, ReplaySource, fetch_tape};
use baylee_client_core::replay::{Header, Perspective, Tape, Verify};
use std::sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, AtomicU8, Ordering}};

impl ReplayHost {
    /// Starts making the tape for `perspective` under `verify`, opening at
    /// `keep`'s input once it is made.
    pub(super) fn start(&mut self, perspective: Perspective, verify: Verify, keep: Option<Kept>) {
        if let Some(old) = self.job.take() {
            old.cancel.store(true, Ordering::Relaxed);
        }
        let progress = Arc::new(AtomicU8::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, answer) = mpsc::channel();
        match &self.source {
            ReplaySource::Record {
                bytes,
                names,
                file,
                by_hand,
            } => {
                let header = Header {
                    tape: baylee_client_core::replay::TAPE_VERSION,
                    view: baylee_view::VIEW_VERSION,
                    build: baylee_build::short().to_string(),
                    source: if *by_hand {
                        baylee_client_core::replay::Source::File
                    } else {
                        baylee_client_core::replay::Source::Local
                    },
                    perspective: baylee_client_core::replay::tape_perspective(perspective),
                    seat: perspective.seat().get(),
                    game: file.clone(),
                    recorded_by: String::new(),
                    own: true,
                    hands: Vec::new(),
                };
                let (bytes, names) = (Arc::clone(bytes), names.clone());
                let (progress, cancel) = (Arc::clone(&progress), Arc::clone(&cancel));
                std::thread::Builder::new()
                    .name("replay".into())
                    .spawn(move || {
                        let tape = super::super::local::project(
                            &bytes,
                            names,
                            header,
                            perspective,
                            verify,
                            &progress,
                            &cancel,
                        );
                        let _ = tx.send(tape);
                    })
                    .ok();
            }
            ReplaySource::Tape { gzip } => {
                let gzip = Arc::clone(gzip);
                std::thread::Builder::new()
                    .name("replay".into())
                    .spawn(move || {
                        let _ = tx.send(Tape::read(&gzip).map_err(|e| e.to_string()));
                    })
                    .ok();
            }
            ReplaySource::Hosted(access) => fetch_tape(access.clone(), perspective, verify, tx),
        }
        self.job = Some(Job {
            answer: Mutex::new(answer),
            progress,
            cancel,
            perspective,
            verify,
            keep,
        });
    }

}
