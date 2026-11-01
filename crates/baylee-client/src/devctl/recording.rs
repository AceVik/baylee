//! Bounded, local-only render recording. No encoding or filesystem I/O runs
//! on the app thread; the two permits include GPU readback and worker writes.

mod http;
mod mp4;
mod writer;

pub(super) use http::{header_line, validate_http};
use writer::write_frames;

use super::{DevControl, Job};
use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Captured, Capturing, Screenshot, ScreenshotCaptured};
use bevy::window::PrimaryWindow;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, channel, sync_channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_OUTSTANDING: usize = 2;
const MAX_FRAME_BYTES: u64 = 32 * 1024 * 1024;
const MAX_RAW_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const READBACK_WARNING: Duration = Duration::from_secs(10);

/// Install only after the loopback listener has started (the parent module
/// itself is native + `dev-control` gated).
pub(super) fn install(app: &mut App) {
    app.init_resource::<Recorder>()
        .add_systems(PreUpdate, service.after(super::pump))
        .add_observer(submitted)
        .add_observer(delivered)
        .init_schedule(RecordFrame)
        .add_systems(RecordFrame, tick);
    // Every normal schedule, including Last, must have applied its Commands
    // before checking for another screenshot. An unordered Last system could
    // miss a feedback / one-shot screenshot still in a deferred command queue.
    app.world_mut()
        .resource_mut::<MainScheduleOrder>()
        .insert_after(Last, RecordFrame);
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct RecordFrame;

#[derive(Component)]
struct RecordedShot {
    run: uuid::Uuid,
    slot: u32,
}

struct Pending {
    requested: Instant,
    stage: Stage,
}

#[derive(Clone, Copy)]
enum Stage {
    Readback {
        entity: Entity,
        submitted: bool,
        delivered: bool,
    },
    Writing,
}

// These observers run on the main world, including when extraction applies
// Capturing. Remember submission/delivery even if Bevy despawns the entity
// before our next tick. A timeout alone never releases a submitted GPU slot.
fn submitted(
    event: On<Add<Capturing>>,
    shots: Query<&RecordedShot>,
    mut recorder: ResMut<Recorder>,
) {
    mark_readback(event.entity, &shots, &mut recorder, false);
}

fn delivered(
    event: On<Add<Captured>>,
    shots: Query<&RecordedShot>,
    mut recorder: ResMut<Recorder>,
) {
    mark_readback(event.entity, &shots, &mut recorder, true);
}

fn mark_readback(
    entity: Entity,
    shots: &Query<&RecordedShot>,
    recorder: &mut Recorder,
    delivery: bool,
) {
    let Ok(shot) = shots.get(entity) else {
        return;
    };
    let Some(run) = recorder
        .run
        .as_mut()
        .filter(|run| run.summary.id == shot.run)
    else {
        return;
    };
    let Some(pending) = run.pending.get_mut(&shot.slot) else {
        return;
    };
    if let Stage::Readback {
        entity: expected,
        submitted,
        delivered,
    } = &mut pending.stage
        && *expected == entity
    {
        *submitted = true;
        *delivered |= delivery;
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Options {
    duration_ms: u32,
    fps: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    video: bool,
}

impl Options {
    fn parse(body: &str) -> Result<Self, String> {
        let options: Self = serde_json::from_str(body).map_err(|e| e.to_string())?;
        if !(1..=30_000).contains(&options.duration_ms) || !(1..=60).contains(&options.fps) {
            return Err("duration_ms must be 1..=30000 and fps 1..=60".into());
        }
        if options.samples() > 900 {
            return Err("at most 900 nominal samples per recording".into());
        }
        Ok(options)
    }

    fn samples(&self) -> u32 {
        (self.duration_ms * self.fps).div_ceil(1000)
    }
}

#[derive(Default, Resource)]
pub(super) struct Recorder {
    run: Option<Run>,
    jobs: Vec<Job>,
}

impl Recorder {
    /// Whether a recording is taking frames now, the time the window has to
    /// be drawn (`window::lift_while_capturing`).
    pub(super) fn capturing(&self) -> bool {
        self.run
            .as_ref()
            .is_some_and(|run| run.summary.phase == "capturing")
    }
}

struct Run {
    summary: Summary,
    start: Option<Instant>,
    next_slot: u32,
    reserved_bytes: u64,
    pending: BTreeMap<u32, Pending>,
    writer: SyncSender<Work>,
    events: Mutex<Receiver<Event>>,
    cancelled: Arc<AtomicBool>,
    final_sent: bool,
}

#[derive(Serialize)]
struct Summary {
    schema: u32,
    id: uuid::Uuid,
    directory: PathBuf,
    phase: &'static str,
    reason: Option<&'static str>,
    error: Option<String>,
    options: Options,
    cadence: &'static str,
    timestamp_note: &'static str,
    started_unix_ms: u128,
    capture_end_us: Option<u64>,
    requested: u32,
    written: u32,
    dropped: u32,
    discarded: u32,
    outstanding: usize,
    reserved_raw_bytes: u64,
    max_outstanding: usize,
    max_frame_bytes: u64,
    max_raw_bytes: u64,
    gaps: Vec<Gap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    video: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    video_step: Option<PathBuf>,
}

#[derive(Serialize)]
struct Gap {
    first_slot: u32,
    count: u32,
    reason: &'static str,
}

#[derive(Serialize)]
struct Sample {
    slot: u32,
    file: String,
    nominal_us: u64,
    requested_us: u64,
    request_frame: u64,
    started_unix_ms: u128,
    virtual_us: u128,
    virtual_speed: f32,
    virtual_paused: bool,
    received_us: u64,
    received_frame: u64,
    width: u32,
    height: u32,
}

enum Work {
    Frame(Sample, Box<Image>),
    Finish(String),
}

enum Event {
    Ready,
    Written(u32, Result<bool, String>),
    Finished(Result<(), String>),
    Failed(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stop {
    #[serde(default)]
    cancel: bool,
}

/// Queue via deferred commands so the input pump needs no recording resource
/// (and the existing headless input harness stays independent of the renderer).
pub(super) fn enqueue(commands: &mut Commands, job: Job) {
    commands.queue(move |world: &mut World| {
        if let Some(mut recorder) = world.get_resource_mut::<Recorder>() {
            recorder.jobs.push(job);
        } else {
            let _ = job
                .reply
                .send(r#"{"error":"recorder is unavailable"}"#.into());
        }
    });
}

fn service(mut recorder: ResMut<Recorder>) {
    for Job { path, body, reply } in std::mem::take(&mut recorder.jobs) {
        let result = match path.as_str() {
            "/recording/start" => start(&mut recorder, &body),
            "/recording/stop" => stop(&mut recorder, &body),
            _ => Ok(()),
        };
        let answer = match result {
            Ok(()) => recorder.run.as_ref().map_or_else(
                || r#"{"ok":true,"phase":"idle"}"#.into(),
                |run| serde_json::json!({"ok": true, "recording": run.summary}).to_string(),
            ),
            Err(error) => serde_json::json!({"error": error}).to_string(),
        };
        let _ = reply.send(answer);
    }
}

fn start(recorder: &mut Recorder, body: &str) -> Result<(), String> {
    let options = Options::parse(body)?;
    if recorder.run.as_ref().is_some_and(|r| {
        !matches!(r.summary.phase, "complete" | "cancelled" | "failed") || !r.pending.is_empty()
    }) {
        return Err("a recording is still active; stop it and wait for drain".into());
    }
    let id = uuid::Uuid::now_v7();
    let directory = std::env::temp_dir().join(format!("baylee-recording-{id}"));
    let summary = Summary {
        schema: 1,
        id,
        directory: directory.clone(),
        phase: "starting",
        reason: None,
        error: None,
        options,
        cadence: "real_monotonic_request_time",
        timestamp_note: "request_frame/virtual_us describe the main-world request; received_* describe async readback delivery, not display/presentation time; nominal_us is the requested schedule; not every displayed frame is captured",
        started_unix_ms: 0,
        capture_end_us: None,
        requested: 0,
        written: 0,
        dropped: 0,
        discarded: 0,
        outstanding: 0,
        reserved_raw_bytes: 0,
        max_outstanding: MAX_OUTSTANDING,
        max_frame_bytes: MAX_FRAME_BYTES,
        max_raw_bytes: MAX_RAW_BYTES,
        gaps: Vec::new(),
        video: None,
        video_step: None,
    };
    let video_fps = summary.options.video.then_some(summary.options.fps);
    let initial = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
    let (writer, jobs) = sync_channel(MAX_OUTSTANDING);
    let (events_tx, events) = channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    std::thread::Builder::new()
        .name("devctl-recording".into())
        .spawn(move || {
            let result = write_frames(
                &directory,
                &initial,
                video_fps,
                &jobs,
                &events_tx,
                &worker_cancelled,
            );
            if let Err(error) = result {
                let _ = events_tx.send(Event::Failed(error));
            }
        })
        .map_err(|e| e.to_string())?;
    recorder.run = Some(Run {
        summary,
        start: None,
        next_slot: 0,
        reserved_bytes: 0,
        pending: BTreeMap::new(),
        writer,
        events: Mutex::new(events),
        cancelled,
        final_sent: false,
    });
    Ok(())
}

fn stop(recorder: &mut Recorder, body: &str) -> Result<(), String> {
    let ask: Stop = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let run = recorder.run.as_mut().ok_or("no recording")?;
    if !matches!(run.summary.phase, "starting" | "capturing" | "draining") {
        return Ok(());
    }
    if run.final_sent {
        return Err("recording is already finalizing; poll status".into());
    }
    if ask.cancel {
        run.cancelled.store(true, Ordering::Relaxed);
    }
    run.end(if ask.cancel { "cancelled" } else { "stopped" });
    Ok(())
}

impl Run {
    fn elapsed_us(&self) -> u64 {
        self.start.map_or(0, |s| micros(s.elapsed()))
    }

    fn end(&mut self, reason: &'static str) {
        if self.summary.capture_end_us.is_none() {
            self.summary.capture_end_us = Some(self.elapsed_us());
        }
        self.summary.phase = "draining";
        self.summary.reason = Some(reason);
    }

    fn gap(&mut self, slot: u32, count: u32, reason: &'static str) {
        if count == 0 {
            return;
        }
        self.summary.dropped += count;
        self.summary.gaps.push(Gap {
            first_slot: slot,
            count,
            reason,
        });
    }

    fn receive(&mut self) {
        // Only the app thread locks this receiver. No worker shares the lock.
        let events: Vec<_> = self.events.lock().unwrap().try_iter().collect();
        for event in events {
            match event {
                Event::Ready => {
                    // Stop/cancel can beat worker initialization. In that case
                    // there was no capture clock: do not reopen it or rewrite
                    // timestamps after a zero-frame summary was queued.
                    if self.summary.phase == "starting" {
                        self.start = Some(Instant::now());
                        self.summary.started_unix_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis();
                        self.summary.phase = "capturing";
                    }
                }
                Event::Written(slot, result) => {
                    self.pending.remove(&slot);
                    match result {
                        Ok(true) => self.summary.written += 1,
                        Ok(false) => self.summary.discarded += 1,
                        Err(error) => {
                            self.gap(slot, 1, "write_failed");
                            self.summary.error = Some(error);
                            self.end("error");
                        }
                    }
                }
                Event::Finished(result) => {
                    self.summary.phase = self.terminal_phase();
                    if let Err(error) = result {
                        self.summary.error = Some(error);
                        self.summary.phase = "failed";
                    }
                }
                Event::Failed(error) => {
                    self.summary.error = Some(error);
                    self.summary.phase = "failed";
                    self.summary.reason = Some("worker_failed");
                }
            }
        }
        self.summary.outstanding = self.pending.len();
    }

    fn recover_lost(&mut self, exists: impl Fn(Entity) -> bool) {
        let lost: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(&slot, pending)| match pending.stage {
                Stage::Readback {
                    entity,
                    submitted,
                    delivered,
                } if delivered || (!submitted && !exists(entity)) => Some(slot),
                _ => None,
            })
            .collect();
        for slot in lost {
            self.pending.remove(&slot);
            self.gap(slot, 1, "callback_lost");
        }
        self.summary.outstanding = self.pending.len();
    }

    fn terminal_phase(&self) -> &'static str {
        if self.summary.error.is_some() {
            "failed"
        } else if self.cancelled.load(Ordering::Relaxed) {
            "cancelled"
        } else {
            "complete"
        }
    }

    fn finish(&mut self) {
        if !self.pending.is_empty() || self.final_sent {
            return;
        }
        self.summary.phase = self.terminal_phase();
        if self.summary.options.video
            && self.summary.written > 0
            && self.summary.phase == "complete"
        {
            self.summary.video = Some(self.summary.directory.join("recording.mp4"));
            self.summary.video_step = Some(self.summary.directory.join("recording-step.mp4"));
        }
        let json = serde_json::to_string_pretty(&self.summary).unwrap();
        self.summary.phase = "draining";
        match self.writer.try_send(Work::Finish(json)) {
            Ok(()) => self.final_sent = true,
            Err(error) => {
                self.summary.error = Some(error.to_string());
                self.summary.phase = "failed";
            }
        }
    }
}

fn tick(
    mut recorder: ResMut<Recorder>,
    mut commands: Commands,
    control: Res<DevControl>,
    clock: Res<Time<Virtual>>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    screenshots: Query<&Screenshot, Without<Capturing>>,
    alive: Query<(), With<RecordedShot>>,
) {
    let Some(run) = recorder.run.as_mut() else {
        return;
    };
    run.receive();
    run.recover_lost(|entity| alive.contains(entity));
    if run
        .pending
        .values()
        .any(|pending| pending.requested.elapsed() > READBACK_WARNING)
        && run.summary.error.is_none()
    {
        run.summary.error = Some("capture/write stalled for 10s; outstanding permits are retained until callbacks drain (restart the client if the renderer never answers)".into());
        run.end("stalled");
    }
    if run.summary.phase == "capturing" {
        schedule(
            run,
            &mut commands,
            control.frame,
            &clock,
            windows.single().ok().map(|(_, window)| window),
            windows.single().is_ok_and(|(primary, _)| {
                let target = Screenshot::window(primary).0.normalize(Some(primary));
                screenshots
                    .iter()
                    .any(|shot| shot.0.normalize(Some(primary)) == target)
            }),
        );
    }
    if run.summary.phase == "draining" {
        run.finish();
    }
}

fn schedule(
    run: &mut Run,
    commands: &mut Commands,
    frame: u64,
    clock: &Time<Virtual>,
    window: Option<&Window>,
    target_busy: bool,
) {
    let elapsed = run.elapsed_us();
    let duration = u64::from(run.summary.options.duration_ms) * 1000;
    let (slot, skipped) = due_slots(
        elapsed,
        run.summary.options.fps,
        run.next_slot,
        run.summary.options.samples(),
    );
    run.gap(run.next_slot, skipped, "cadence_missed");
    run.next_slot += skipped;
    if elapsed >= duration {
        if let Some(slot) = slot {
            run.gap(slot, 1, "cadence_missed");
            run.next_slot += 1;
        }
        run.end("duration");
        return;
    }
    let Some(slot) = slot else {
        return;
    };
    run.next_slot = slot + 1;
    if target_busy {
        run.gap(slot, 1, "target_busy");
        return;
    }
    if run.pending.len() >= MAX_OUTSTANDING {
        run.gap(slot, 1, "backpressure");
        return;
    }
    let Some(window) = window else {
        run.gap(slot, 1, "no_window");
        run.end("no_window");
        return;
    };
    let bytes = (u64::from(window.physical_width()) * u64::from(window.physical_height()))
        .saturating_mul(4);
    if bytes == 0 || bytes > MAX_FRAME_BYTES || bytes + run.reserved_bytes > MAX_RAW_BYTES {
        run.gap(slot, 1, "size_budget");
        run.end("size_budget");
        return;
    }
    run.reserved_bytes += bytes;
    run.summary.reserved_raw_bytes = run.reserved_bytes;
    run.summary.requested += 1;
    let metadata = Sample {
        slot,
        file: format!("frame-{slot:06}.png"),
        nominal_us: u64::from(slot) * 1_000_000 / u64::from(run.summary.options.fps),
        requested_us: elapsed,
        request_frame: frame,
        started_unix_ms: run.summary.started_unix_ms,
        virtual_us: clock.elapsed().as_micros(),
        virtual_speed: clock.relative_speed(),
        virtual_paused: clock.is_paused(),
        received_us: 0,
        received_frame: 0,
        width: window.physical_width(),
        height: window.physical_height(),
    };
    let entity = commands
        .spawn((
            Screenshot::primary_window(),
            RecordedShot {
                run: run.summary.id,
                slot,
            },
        ))
        .observe(captured(run.summary.id, metadata))
        .id();
    run.pending.insert(
        slot,
        Pending {
            requested: Instant::now(),
            stage: Stage::Readback {
                entity,
                submitted: false,
                delivered: false,
            },
        },
    );
    run.summary.outstanding = run.pending.len();
}

/// Consume overdue nominal slots without ever issuing a catch-up burst.
fn due_slots(elapsed_us: u64, fps: u32, next: u32, total: u32) -> (Option<u32>, u32) {
    let due =
        (elapsed_us.saturating_mul(u64::from(fps)) / 1_000_000).min(u64::from(total - 1)) as u32;
    if next > due {
        (None, 0)
    } else {
        (Some(due), due - next)
    }
}

fn captured(
    run_id: uuid::Uuid,
    metadata: Sample,
) -> impl FnMut(On<ScreenshotCaptured>, ResMut<Recorder>, Res<DevControl>) {
    let mut metadata = Some(metadata);
    move |mut event, mut recorder, control| {
        let Some(mut metadata) = metadata.take() else {
            return;
        };
        let Some(run) = recorder.run.as_mut().filter(|run| run.summary.id == run_id) else {
            return;
        };
        let slot = metadata.slot;
        let Some(pending) = run.pending.get_mut(&slot) else {
            return;
        };
        if !matches!(pending.stage, Stage::Readback { entity, .. } if entity == event.entity) {
            return;
        }
        pending.stage = Stage::Writing;
        if run.cancelled.load(Ordering::Relaxed) || run.summary.phase == "failed" {
            run.pending.remove(&slot);
            run.summary.discarded += 1;
            return;
        }
        metadata.received_us = run.elapsed_us();
        metadata.received_frame = control.frame;
        // A resize between request and readback must not bypass our byte cap.
        let size = event.image.data.as_ref().map_or(0, Vec::len) as u64;
        if size == 0
            || size > MAX_FRAME_BYTES
            || event.image.width() != metadata.width
            || event.image.height() != metadata.height
        {
            run.pending.remove(&slot);
            run.gap(slot, 1, "resized_or_oversize");
            run.end("size_changed");
            return;
        }
        // This observer owns a dedicated screenshot entity. Move its buffer,
        // rather than cloning tens of MB on the main thread.
        let image = std::mem::take(&mut event.event_mut().image);
        if let Err(error) = run.writer.try_send(Work::Frame(metadata, Box::new(image))) {
            run.pending.remove(&slot);
            run.gap(slot, 1, "writer_unavailable");
            run.summary.error = Some(error.to_string());
            run.end("error");
        }
    }
}

fn micros(duration: Duration) -> u64 {
    duration.as_micros().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
