//! The recorded soundtrack's worker and the ring it fills.
//!
//! The Opus decoder allocates per packet, so it never runs on the audio
//! thread. One parked thread per playing `Tune` runs the [`Mixer`] ahead of
//! the device into a fixed single-producer, single-consumer ring of stereo
//! frames; the audio thread only loads atomics out of it ([`Feed::add`]). The
//! ring holds [`CAPACITY`] frames (≈ 170 ms), which is also how late the
//! recorded music answers a request. A paused sink stops taking frames, the
//! ring stays full and the worker sleeps; the music resumes where it stood.
use super::Mixer;
use crate::music::{ScoreControl, orchestra::BLOCK};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
    },
    time::Duration,
};

/// Frames the ring holds.
pub(crate) const CAPACITY: usize = 8192;

/// A lock-free ring of stereo frames, `f32` bits in atomics: safe code, no
/// lock either side can be made to wait on.
pub(crate) struct Ring {
    samples: Box<[AtomicU32]>,
    /// Frames written and read since the start, wrapping.
    written: AtomicUsize,
    read: AtomicUsize,
    closed: AtomicBool,
    /// Frames a block wanted after the ring ran dry in the middle of it.
    pub(crate) starved: AtomicUsize,
}

impl Ring {
    pub(crate) fn new() -> Self {
        Self {
            samples: (0..CAPACITY * 2).map(|_| AtomicU32::new(0)).collect(),
            written: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
            starved: AtomicUsize::new(0),
        }
    }

    /// Room for this many frames (the writer's view).
    pub(crate) fn room(&self) -> usize {
        let written = self.written.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        CAPACITY - written.wrapping_sub(read)
    }

    /// Writes as many frames as there is room for; the writer only.
    pub(crate) fn push(&self, frames: &[[f32; 2]]) -> usize {
        let written = self.written.load(Ordering::Relaxed);
        let count = frames.len().min(self.room());
        for (k, frame) in frames[..count].iter().enumerate() {
            let at = written.wrapping_add(k) % CAPACITY * 2;
            self.samples[at].store(frame[0].to_bits(), Ordering::Relaxed);
            self.samples[at + 1].store(frame[1].to_bits(), Ordering::Relaxed);
        }
        self.written
            .store(written.wrapping_add(count), Ordering::Release);
        count
    }

    /// Adds as many frames as the ring holds to `out`; the reader only.
    /// What it does not have is silence.
    pub(crate) fn add(&self, out: &mut [[f32; 2]]) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let ready = self.written.load(Ordering::Acquire).wrapping_sub(read);
        let count = out.len().min(ready);
        for (k, frame) in out[..count].iter_mut().enumerate() {
            let at = read.wrapping_add(k) % CAPACITY * 2;
            frame[0] += f32::from_bits(self.samples[at].load(Ordering::Relaxed));
            frame[1] += f32::from_bits(self.samples[at + 1].load(Ordering::Relaxed));
        }
        self.read.store(read.wrapping_add(count), Ordering::Release);
        // An empty ring is the worker resting; one that runs dry
        // mid-block is an underrun.
        if count > 0 && count < out.len() {
            self.starved.fetch_add(out.len() - count, Ordering::Relaxed);
        }
        count
    }
}

/// The audio thread's end of a running worker. Dropping it stops the worker.
pub(crate) struct Feed {
    ring: Arc<Ring>,
}

impl Feed {
    /// Starts a worker that performs what `control` asks for; `None` where
    /// no thread can be started (the synthesised score plays on alone).
    pub(crate) fn spawn(control: Arc<ScoreControl>) -> Option<Self> {
        let ring = Arc::new(Ring::new());
        let theirs = ring.clone();
        std::thread::Builder::new()
            .name("baylee-score".into())
            .spawn(move || work(&theirs, &control))
            .ok()?;
        Some(Self { ring })
    }

    /// Adds the recorded music's next frames to `out`. Loads only: no lock,
    /// no allocation, no system call.
    pub(crate) fn add(&self, out: &mut [[f32; 2]]) {
        self.ring.add(out);
    }

    /// Frames the reader found missing so far (an underrun is silence).
    #[must_use]
    pub(crate) fn starved(&self) -> usize {
        self.ring.starved.load(Ordering::Relaxed)
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.ring.closed.store(true, Ordering::Relaxed);
    }
}

fn work(ring: &Ring, control: &ScoreControl) {
    let mut mixer = Mixer::new();
    let mut block = [[0.0f32; 2]; BLOCK];
    while !ring.closed.load(Ordering::Relaxed) {
        let mut wrote = false;
        loop {
            let request = control.get();
            // Silent and asked for nothing: an empty ring is silence too.
            if !request.recorded && mixer.silent() {
                break;
            }
            if ring.room() < BLOCK {
                break;
            }
            mixer.render(request, &mut block);
            ring.push(&block);
            wrote = true;
        }
        std::thread::park_timeout(Duration::from_millis(if wrote { 10 } else { 40 }));
    }
}
