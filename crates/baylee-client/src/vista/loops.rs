//! The front door's two moving pictures: the world behind the sign-in and
//! the tavern behind the lobby.
//!
//! Each is an AVIF image sequence of our own (`loops/world.avif`,
//! `loops/tavern.avif`: 1280 × 720, 60/1 frames a second, `2 × 2` tiles,
//! low-delay 1-frame-per-sample; `art/baylee/README.md` has their provenance
//! and the `ffmpeg` command that made them), shipped inside the binary and
//! decoded by `baylee-avif` with ARM64 NEON on Apple Silicon.
//!
//! How a loop plays:
//!
//! - **Exact real-time tempo.** [`step`] advances each loop's playhead from
//!   [`super::VistaClock`] (`Time<Real>` nanoseconds, never frame count), and
//!   each sample's presentation time is computed directly from the track's
//!   rational `(ticks, timescale)` without integer-millisecond truncation.
//!   Whether the window renders at 30, 60 or 120 fps, the animation runs at
//!   exact `1.0×` wall-clock tempo with zero drift over arbitrary hours.
//! - **Lock-free SPSC lookahead.** One background worker per loop decodes
//!   `256`-byte row-aligned YUV420 planes directly into a pre-allocated 6-slot
//!   single-producer/single-consumer ring (`0` per-frame heap allocations, no
//!   `Mutex`), staying up to 6 frames (`100` ms) ahead of the playhead and
//!   parking whenever the ring is full or the window is paused.
//! - **Zero-churn GPU upload.** In `RenderSystems::PrepareAssets`, [`copy`]
//!   writes the due frame into a persistent pool of reusable `MAP_WRITE |
//!   COPY_SRC` staging buffers and issues a single `CommandEncoder` with three
//!   256-byte pitch-aligned `copy_buffer_to_texture` DMA transfers.
//! - **Pre-warmed transitions & clean eviction.** Entering the sign-in form or
//!   auth wait pre-warms `tavern.avif` off-thread before the portal opens. An
//!   unseen loop parks immediately and drops its decoder after [`KEEP`],
//!   resetting [`Loop::showing`] and bumping its generation so stale GPU frames
//!   never flash.
//!
//! The files are ours and are read under `Bounds::SHIPPED_LOOP`; no file from
//! outside reaches this decoder (`baylee-avif`'s own docs say why).

use std::cell::UnsafeCell;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use baylee_avif::{AlignedPlanes, Bounds, Matrix, Player, Sequence};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode, PollType,
    TexelCopyBufferInfo, TexelCopyBufferLayout, TextureDimension, TextureFormat,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::texture::GpuImage;

use super::{FrontScene, Vista, VistaClock, VistaMaterial};

/// The world behind the sign-in.
const WORLD: &[u8] = include_bytes!("loops/world.avif");
/// The tavern behind the lobby.
const TAVERN: &[u8] = include_bytes!("loops/tavern.avif");

/// How long a loop takes to come in over its still.
const FADE_IN: f32 = 0.6;

/// How long a loop nobody sees keeps its decoder before freeing it.
const KEEP: Duration = Duration::from_secs(4);

/// Pre-allocated lookahead slots in the lock-free SPSC ring (`100` ms at 60 fps).
const RING_CAP: usize = 6;

/// Tile-parallel worker threads (`2 × 2` AV1 tiles).
const DECODE_THREADS: u32 = 4;

/// Maximum lag (`250` ms) before re-anchoring stream ticks to `playhead_ns`.
const MAX_BEHIND_NS: u64 = 250_000_000;

/// Fallback park timeout when waiting on a full ring.
const NAP: Duration = Duration::from_millis(250);

const FIRST_EMPTY: u8 = 0;
const FIRST_READY: u8 = 1;
const FIRST_UPLOADED: u8 = 2;

const CHUNK_MAPPED: u8 = 0;
const CHUNK_PENDING: u8 = 1;
const CHUNK_FAILED: u8 = 2;

/// A loop's planes on the GPU: the running frame's Y, U and V, then the
/// first frame's.
pub(crate) type Planes = [Handle<Image>; 6];

/// One pre-allocated slot in [`FrameRing`].
struct RingSlot {
    planes: AlignedPlanes,
    stream_pts_ns: u64,
    generation: u64,
}

/// Pre-allocated lock-free single-producer / single-consumer frame ring.
struct FrameRing {
    slots: [UnsafeCell<RingSlot>; RING_CAP],
    /// Monotonic count of slots published by the worker thread.
    write_pos: AtomicU64,
    /// Monotonic count of slots consumed by the render-world uploader.
    read_pos: AtomicU64,
    /// Pre-allocated frame-0 buffer for the 4K-still residual mask.
    first: UnsafeCell<AlignedPlanes>,
    /// Lifecycle of `first`: `FIRST_EMPTY -> FIRST_READY -> FIRST_UPLOADED`.
    first_state: AtomicU8,
}

// SAFETY: `FrameRing` enforces single-producer (the `Loop`'s sole worker
// thread) and single-consumer (`loops::copy` in `RenderSystems::PrepareAssets`,
// or the single test thread in headless unit tests) ownership via `Acquire` /
// `Release` atomic cursors `write_pos`, `read_pos`, and `first_state`. No slot
// is ever accessed concurrently by producer and consumer.
unsafe impl Sync for FrameRing {}

impl FrameRing {
    fn new(width: u32, height: u32) -> Self {
        let slots = std::array::from_fn(|_| {
            UnsafeCell::new(RingSlot {
                planes: AlignedPlanes::with_dimensions(width, height),
                stream_pts_ns: 0,
                generation: 0,
            })
        });
        Self {
            slots,
            write_pos: AtomicU64::new(0),
            read_pos: AtomicU64::new(0),
            first: UnsafeCell::new(AlignedPlanes::with_dimensions(width, height)),
            first_state: AtomicU8::new(FIRST_EMPTY),
        }
    }
}

/// One loop: its textures, exact rational timing, atomic playhead and SPSC ring.
pub(crate) struct Loop {
    /// Whether this is the tavern's (else the world's).
    tavern: bool,
    planes: Planes,
    size: UVec2,
    timescale: NonZeroU64,
    durations_ticks: Vec<u64>,
    total_ticks: u64,
    /// Monotonic stream playhead in nanoseconds from `origin_clock_ns`.
    playhead_ns: AtomicU64,
    /// `VistaClock::nanos` when the current active playback epoch started.
    origin_clock_ns: AtomicU64,
    /// Whether `origin_clock_ns` has been latched for the current epoch.
    anchored: AtomicBool,
    /// Incremented whenever the decoder resets so stale ring slots are skipped.
    generation: AtomicU64,
    /// A visible surface is actively showing this loop.
    active: AtomicBool,
    /// Pre-warm requested off-thread ahead of a portal transition.
    warm: AtomicBool,
    /// Paused while minimised, occluded, or frozen (`0%` CPU/GPU).
    paused: AtomicBool,
    stop: AtomicBool,
    /// Both `first` and a current-generation running frame are on the GPU.
    showing: AtomicBool,
    /// The decoder refused the file: the still stands for the session.
    failed: AtomicBool,
    ring: FrameRing,
    worker: OnceLock<std::thread::Thread>,
}

impl Drop for Loop {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.get() {
            worker.unpark();
        }
    }
}

/// Exact conversion from track ticks to nanoseconds without intermediate overflow.
#[inline]
fn ticks_to_ns(ticks: u64, timescale: NonZeroU64) -> u64 {
    #[allow(clippy::cast_possible_truncation)]
    let ns = (u128::from(ticks) * 1_000_000_000 / u128::from(timescale.get())) as u64;
    ns
}

impl Loop {
    /// Whether a surface of this kind draws this loop.
    fn plays(&self, kind: Vista) -> bool {
        self.tavern == (kind == Vista::Interior)
    }

    /// Whether the loop is on the GPU and may be drawn.
    pub(crate) fn showing(&self) -> bool {
        self.showing.load(Ordering::Acquire)
    }

    /// Reads a shipped file's tables, pre-allocates its textures and SPSC ring,
    /// and spawns its parked worker thread.
    fn start(tavern: bool, bytes: &'static [u8], images: &mut Assets<Image>) -> Option<Arc<Self>> {
        Self::start_with_keep(tavern, bytes, images, KEEP)
    }

    fn start_with_keep(
        tavern: bool,
        bytes: &'static [u8],
        images: &mut Assets<Image>,
        keep: Duration,
    ) -> Option<Arc<Self>> {
        let sequence = Sequence::read(bytes, &Bounds::SHIPPED_LOOP).ok()?;
        let track = &sequence.colour;
        if track.samples.is_empty() || track.total_ticks == 0 {
            return None;
        }
        let size = UVec2::new(track.width, track.height);
        let chroma = (size + UVec2::ONE) / 2;
        let durations_ticks: Vec<u64> = track.samples.iter().map(|s| s.duration_ticks).collect();
        let mut make_plane = |plane_size: UVec2, fill: u8| {
            images.add(Image::new_fill(
                Extent3d {
                    width: plane_size.x,
                    height: plane_size.y,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[fill],
                TextureFormat::R8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            ))
        };
        // Black in limited-range BT.709: luma 16, chroma 128.
        let planes = [
            make_plane(size, 16),
            make_plane(chroma, 128),
            make_plane(chroma, 128),
            make_plane(size, 16),
            make_plane(chroma, 128),
            make_plane(chroma, 128),
        ];
        let lp = Arc::new(Self {
            tavern,
            planes,
            size,
            timescale: track.timescale,
            durations_ticks,
            total_ticks: track.total_ticks,
            playhead_ns: AtomicU64::new(0),
            origin_clock_ns: AtomicU64::new(0),
            anchored: AtomicBool::new(false),
            generation: AtomicU64::new(1),
            active: AtomicBool::new(false),
            warm: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            showing: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            ring: FrameRing::new(size.x, size.y),
            worker: OnceLock::new(),
        });
        let theirs = Arc::downgrade(&lp);
        let worker = std::thread::Builder::new()
            .name(if tavern { "tavern-loop" } else { "world-loop" }.to_string())
            .spawn(move || work(&theirs, bytes, keep))
            .ok()?;
        let _ = lp.worker.set(worker.thread().clone());
        Some(lp)
    }

    /// Gives the loop up for the session: its still stands.
    fn fail(&self, why: &dyn std::fmt::Display) {
        if !self.failed.swap(true, Ordering::AcqRel) {
            warn!(
                "vista: the {} loop does not play: {why}",
                if self.tavern { "tavern" } else { "world" }
            );
        }
    }
}

/// Drops an unseen decoder after `keep`, resetting `showing` and `generation`
/// so stale GPU frames never flash when the scene is entered again.
fn evict_player(lp: &Loop, player: &mut Option<Player>, stream_ticks: &mut u64) {
    if player.is_some() {
        *player = None;
        *stream_ticks = 0;
        lp.showing.store(false, Ordering::Release);
        lp.anchored.store(false, Ordering::Release);
        lp.playhead_ns.store(0, Ordering::Release);
        lp.generation.fetch_add(1, Ordering::AcqRel);
    }
}

/// Decodes one aligned sample into `ring.slots[slot_idx]` and publishes it.
fn decode_into_slot(
    lp: &Loop,
    decoder: &mut Player,
    write_pos: u64,
    generation: u64,
    stream_ticks: &mut u64,
) -> bool {
    let playhead = lp.playhead_ns.load(Ordering::Acquire);
    let next_pts = ticks_to_ns(*stream_ticks, lp.timescale);
    if playhead > next_pts.saturating_add(MAX_BEHIND_NS) {
        let target_ticks = (u128::from(playhead) * u128::from(lp.timescale.get())) / 1_000_000_000;
        let cur = u128::from(*stream_ticks);
        let total = u128::from(lp.total_ticks.max(1));
        let jumps = (target_ticks.saturating_sub(cur)) / total;
        #[allow(clippy::cast_possible_truncation)]
        if jumps > 0 {
            *stream_ticks = stream_ticks.saturating_add((jumps * total) as u64);
        }
    }
    let pts_ns = ticks_to_ns(*stream_ticks, lp.timescale);
    let slot_idx = (write_pos % RING_CAP as u64) as usize;
    // SAFETY: Only this single worker thread writes `ring.slots` and updates
    // `write_pos`. The caller verified `write_pos - read_pos < RING_CAP` with
    // `Acquire` on `read_pos`, so `slot_idx` is not readable by the consumer
    // until `write_pos.store(write_pos + 1, Release)` below.
    let slot = unsafe { &mut *lp.ring.slots[slot_idx].get() };
    let index = match decoder.advance_aligned(&mut slot.planes) {
        Ok(step) => step,
        Err(e) => {
            lp.fail(&e);
            return false;
        }
    };
    let fits = slot.planes.width == lp.size.x
        && slot.planes.height == lp.size.y
        && matches!(slot.planes.matrix, Matrix::Bt709)
        && !slot.planes.full_range;
    if !fits {
        lp.fail(&"a frame is not 8-bit limited-range BT.709 of the track's size");
        return false;
    }
    if index == 0 && lp.ring.first_state.load(Ordering::Acquire) == FIRST_EMPTY {
        // SAFETY: Only the worker writes `ring.first`, strictly while
        // `first_state == FIRST_EMPTY` before publishing `FIRST_READY` with
        // `Release`; the consumer reads `ring.first` only after observing
        // `FIRST_READY` with `Acquire`.
        let first = unsafe { &mut *lp.ring.first.get() };
        first.clone_from(&slot.planes);
        lp.ring.first_state.store(FIRST_READY, Ordering::Release);
    }
    let dur = lp.durations_ticks.get(index).copied().unwrap_or(0);
    *stream_ticks = stream_ticks.saturating_add(dur);
    slot.stream_pts_ns = pts_ns;
    slot.generation = generation;
    lp.ring.write_pos.store(write_pos + 1, Ordering::Release);
    true
}

/// Background decode worker: fills the 6-slot lookahead ring while active or
/// warming, parks immediately when full or paused, and frees its decoder after
/// `keep` unseen.
fn work(lp: &std::sync::Weak<Loop>, bytes: &'static [u8], keep: Duration) {
    let mut player: Option<Player> = None;
    let mut stream_ticks = 0u64;
    let mut active_gen = 0u64;
    let mut unseen_since: Option<Instant> = None;
    loop {
        let Some(lp) = lp.upgrade() else {
            return;
        };
        if lp.stop.load(Ordering::Acquire) {
            return;
        }
        let needed = lp.active.load(Ordering::Acquire) || lp.warm.load(Ordering::Acquire);
        if !needed || lp.failed.load(Ordering::Acquire) {
            let since = *unseen_since.get_or_insert_with(Instant::now);
            let waited = since.elapsed();
            if waited >= keep {
                evict_player(&lp, &mut player, &mut stream_ticks);
                drop(lp);
                std::thread::park();
            } else {
                drop(lp);
                std::thread::park_timeout(keep.saturating_sub(waited));
            }
            continue;
        }
        unseen_since = None;
        if lp.paused.load(Ordering::Acquire) {
            drop(lp);
            std::thread::park();
            continue;
        }
        if player.is_none() {
            match Player::open_static(bytes, &Bounds::SHIPPED_LOOP, DECODE_THREADS) {
                Ok(opened) => {
                    stream_ticks = 0;
                    active_gen = lp.generation.load(Ordering::Acquire);
                    player = Some(opened);
                }
                Err(e) => {
                    lp.fail(&e);
                    continue;
                }
            }
        }
        let gen_now = lp.generation.load(Ordering::Acquire);
        if gen_now != active_gen {
            active_gen = gen_now;
            stream_ticks = 0;
        }
        let w = lp.ring.write_pos.load(Ordering::Relaxed);
        let r = lp.ring.read_pos.load(Ordering::Acquire);
        if (w.saturating_sub(r) as usize) >= RING_CAP {
            drop(lp);
            std::thread::park_timeout(NAP);
            continue;
        }
        if let Some(decoder) = player.as_mut() {
            let _ = decode_into_slot(&lp, decoder, w, active_gen, &mut stream_ticks);
        }
    }
}

/// The loops, in the main world and the render world alike.
#[derive(Resource, Clone)]
pub(crate) struct Loops(Arc<[Arc<Loop>]>);

impl Loops {
    /// The loop a surface of this kind draws.
    fn of(&self, kind: Vista) -> Option<&Arc<Loop>> {
        self.0.iter().find(|lp| lp.plays(kind))
    }
}

/// Starts both loops, parked, and wires the render-world DMA uploader.
pub(super) fn install(app: &mut App) {
    let Some(mut images) = app.world_mut().get_resource_mut::<Assets<Image>>() else {
        return;
    };
    let loops: Vec<Arc<Loop>> = [(false, WORLD), (true, TAVERN)]
        .into_iter()
        .filter_map(|(tavern, bytes)| Loop::start(tavern, bytes, &mut images))
        .collect();
    if loops.is_empty() {
        return;
    }
    let loops = Loops(Arc::from(loops));
    app.insert_resource(loops.clone())
        .add_systems(Update, (step, light).chain().after(super::paint));
    if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp) {
        render.insert_resource(loops).add_systems(
            bevy::render::Render,
            copy.in_set(bevy::render::RenderSystems::PrepareAssets)
                .after(bevy::render::render_asset::prepare_assets::<GpuImage>),
        );
    }
}

/// Updates each loop's visibility, pre-warm hint, pause state, and exact
/// real-nanosecond playhead from [`VistaClock`].
#[allow(clippy::too_many_arguments)]
fn step(
    loops: Res<Loops>,
    clock: Option<Res<VistaClock>>,
    resting: Option<Res<crate::quality::Resting>>,
    front: Option<Res<FrontScene>>,
    surfaces: Query<(
        &Vista,
        &MaterialNode<VistaMaterial>,
        &Visibility,
        &InheritedVisibility,
    )>,
    materials: Option<Res<Assets<VistaMaterial>>>,
    mut moving: ResMut<super::Moving>,
) {
    let Some(materials) = materials else {
        return;
    };
    let clock_val = clock.as_deref().copied().unwrap_or_default();
    let is_paused = !clock_val.energy || resting.as_deref().is_some_and(|r| r.0);
    let has_interior = surfaces.iter().any(|(kind, ..)| *kind == Vista::Interior);
    let has_wait = surfaces
        .iter()
        .any(|(kind, _, own, inh)| *kind == Vista::Wait && *own != Visibility::Hidden && inh.get());
    let warm_tavern = clock_val.energy
        && has_interior
        && (has_wait
            || front
                .as_deref()
                .is_some_and(|f| f.entering || !f.shown || f.stage.hour > 0.0));

    let mut any_active = false;
    for lp in loops.0.iter() {
        let visible = clock_val.energy
            && surfaces.iter().any(|(kind, handle, own, inherited)| {
                lp.plays(*kind)
                    && *own != Visibility::Hidden
                    && inherited.get()
                    && materials
                        .get(&handle.0)
                        .is_some_and(|m| m.params.view.w > 0.0 && m.params.air.z > 0.0)
            });
        let warm = lp.tavern && warm_tavern && !visible;
        any_active |= visible && !lp.failed.load(Ordering::Relaxed);

        let mut wake = lp.paused.swap(is_paused, Ordering::AcqRel) != is_paused;
        wake |= lp.active.swap(visible, Ordering::AcqRel) != visible;
        wake |= lp.warm.swap(warm, Ordering::AcqRel) != warm;

        if visible {
            if !lp.anchored.swap(true, Ordering::AcqRel) {
                lp.origin_clock_ns.store(clock_val.nanos, Ordering::Release);
            }
            let origin = lp.origin_clock_ns.load(Ordering::Acquire);
            let playhead = clock_val.nanos.saturating_sub(origin);
            wake |= lp.playhead_ns.swap(playhead, Ordering::AcqRel) != playhead;
        } else if !warm {
            lp.anchored.store(false, Ordering::Release);
        }

        if wake
            && !is_paused
            && let Some(worker) = lp.worker.get()
        {
            worker.unpark();
        }
    }
    moving.set_if_neq(super::Moving(any_active));
}

/// Wires each surface's six plane handles once, and fades `moving.x` in once
/// [`Loop::showing`] reports that `first` and a running frame are on the GPU.
fn light(
    real_time: Option<Res<Time<Real>>>,
    time: Res<Time>,
    loops: Res<Loops>,
    surfaces: Query<(&Vista, &MaterialNode<VistaMaterial>)>,
    materials: Option<ResMut<Assets<VistaMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = real_time
        .as_deref()
        .map_or_else(|| time.delta_secs(), Time::delta_secs);
    let fade_step = dt / FADE_IN;
    for (kind, handle) in &surfaces {
        let Some(lp) = loops.of(*kind) else {
            continue;
        };
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        let now = material.params.moving.x;
        let next = super::towards(now, if lp.showing() { 1.0 } else { 0.0 }, fade_step);
        let wired = material
            .planes()
            .zip(&lp.planes)
            .all(|(a, b)| a.id() == b.id());
        #[allow(clippy::float_cmp)] // `towards` lands exactly on `0.0` or `1.0`
        let moved = next != now;
        if (moved || !wired)
            && let Some(mut material) = materials.get_mut(&handle.0)
        {
            material.params.moving.x = next;
            if !wired {
                material.wire(&lp.planes);
            }
        }
    }
}

/// Persistent reusable `MAP_WRITE | COPY_SRC` staging buffer chunk.
struct StagingChunk {
    buffer: Buffer,
    size: u64,
    state: Arc<AtomicU8>,
}

/// Render-world pool of reusable mapped staging buffers (`0` allocations per
/// frame after warm-up).
#[derive(Default)]
struct StagingPool {
    chunks: Vec<StagingChunk>,
}

impl StagingPool {
    fn reclaim(&mut self, device: &RenderDevice) {
        let _ = device.poll(PollType::Poll);
        self.chunks
            .retain(|c| c.state.load(Ordering::Acquire) != CHUNK_FAILED);
    }

    fn acquire(&mut self, device: &RenderDevice, size: u64) -> usize {
        if let Some(idx) = self
            .chunks
            .iter()
            .position(|c| c.size == size && c.state.load(Ordering::Acquire) == CHUNK_MAPPED)
        {
            return idx;
        }
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("vista-avif-staging"),
            size,
            usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        self.chunks.push(StagingChunk {
            buffer,
            size,
            state: Arc::new(AtomicU8::new(CHUNK_MAPPED)),
        });
        self.chunks.len() - 1
    }
}

/// Uploads one `AlignedPlanes` buffer to three `R8Unorm` GPU textures in a
/// single `CommandEncoder` using a persistent mapped staging buffer.
fn upload_aligned(
    images: &RenderAssets<GpuImage>,
    device: &RenderDevice,
    queue: &RenderQueue,
    pool: &mut StagingPool,
    targets: &[Handle<Image>],
    planes: &AlignedPlanes,
) -> bool {
    let size = planes.data.len() as u64;
    if size == 0 {
        return false;
    }
    let cw = planes.chroma_width();
    let ch = planes.chroma_height();
    let specs = [
        (
            &targets[0],
            0u64,
            planes.y_stride,
            planes.width,
            planes.height,
        ),
        (
            &targets[1],
            planes.u_offset() as u64,
            planes.c_stride,
            cw,
            ch,
        ),
        (
            &targets[2],
            planes.v_offset() as u64,
            planes.c_stride,
            cw,
            ch,
        ),
    ];
    let ready = specs.iter().all(|(h, _, _, w, height)| {
        images.get(h.id()).is_some_and(|gpu| {
            let ext = gpu.texture_descriptor.size;
            ext.width == *w
                && ext.height == *height
                && gpu.texture_descriptor.format == TextureFormat::R8Unorm
        })
    });
    if !ready {
        return false;
    }

    let idx = pool.acquire(device, size);
    let chunk = &pool.chunks[idx];
    let Ok(mut mapped) = chunk.buffer.slice(..size).get_mapped_range_mut() else {
        chunk.state.store(CHUNK_FAILED, Ordering::Release);
        return false;
    };
    mapped.copy_from_slice(&planes.data);
    drop(mapped);
    chunk.buffer.unmap();
    chunk.state.store(CHUNK_PENDING, Ordering::Relaxed);

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("vista-avif-upload"),
    });
    for (handle, offset, stride, width, height) in specs {
        let Some(gpu) = images.get(handle.id()) else {
            return false;
        };
        encoder.copy_buffer_to_texture(
            TexelCopyBufferInfo {
                buffer: &chunk.buffer,
                layout: TexelCopyBufferLayout {
                    offset,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            gpu.texture.as_image_copy(),
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
    queue.submit(std::iter::once(encoder.finish()));

    let flag = Arc::clone(&chunk.state);
    device.map_buffer(&chunk.buffer.slice(..size), MapMode::Write, move |res| {
        flag.store(
            if res.is_ok() {
                CHUNK_MAPPED
            } else {
                CHUNK_FAILED
            },
            Ordering::Release,
        );
    });
    true
}

/// Selects the newest due frame (`stream_pts_ns <= playhead_ns`) from `lp.ring`,
/// discarding any stale-generation slots ahead of it.
fn select_due_slot(lp: &Loop) -> Option<u64> {
    let initial_r = lp.ring.read_pos.load(Ordering::Relaxed);
    let mut r = initial_r;
    let w = lp.ring.write_pos.load(Ordering::Acquire);
    let cur_gen = lp.generation.load(Ordering::Acquire);
    while r < w {
        let idx = (r % RING_CAP as u64) as usize;
        // SAFETY: `r < w` after `Acquire` on `write_pos` guarantees the worker
        // finished writing `slots[idx]` and will not touch it until `read_pos`
        // advances past `r`.
        let slot_gen = unsafe { (*lp.ring.slots[idx].get()).generation };
        if slot_gen == cur_gen {
            break;
        }
        r += 1;
    }
    if r != initial_r {
        lp.ring.read_pos.store(r, Ordering::Release);
        if let Some(worker) = lp.worker.get() {
            worker.unpark();
        }
    }
    if r >= w {
        return None;
    }
    let playhead = lp.playhead_ns.load(Ordering::Acquire);
    let mut chosen = None;
    let mut pos = r;
    while pos < w {
        let idx = (pos % RING_CAP as u64) as usize;
        // SAFETY: Same SPSC invariant for `pos` in `r..w`.
        let (slot_gen, pts) = unsafe {
            let s = &*lp.ring.slots[idx].get();
            (s.generation, s.stream_pts_ns)
        };
        if slot_gen != cur_gen {
            break;
        }
        if pts <= playhead || (!lp.showing.load(Ordering::Relaxed) && chosen.is_none()) {
            chosen = Some(pos);
            pos += 1;
        } else {
            break;
        }
    }
    chosen
}

/// Render-world system: uploads `first` once and the latest due frame from the
/// SPSC lookahead ring into persistent `R8Unorm` textures.
fn copy(
    loops: Option<Res<Loops>>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut pool: Local<StagingPool>,
) {
    let Some(loops) = loops else {
        return;
    };
    pool.reclaim(&device);
    for lp in loops.0.iter() {
        if lp.paused.load(Ordering::Acquire)
            || (!lp.active.load(Ordering::Acquire) && !lp.warm.load(Ordering::Acquire))
        {
            continue;
        }
        let mut first_done = lp.ring.first_state.load(Ordering::Acquire) == FIRST_UPLOADED;
        if lp.ring.first_state.load(Ordering::Acquire) == FIRST_READY {
            // SAFETY: Observing `FIRST_READY` with `Acquire` synchronizes-with
            // the worker's `Release` store after writing `ring.first`, and the
            // worker never mutates `ring.first` again.
            let first = unsafe { &*lp.ring.first.get() };
            if upload_aligned(&images, &device, &queue, &mut pool, &lp.planes[3..], first) {
                lp.ring.first_state.store(FIRST_UPLOADED, Ordering::Release);
                first_done = true;
            }
        }
        let Some(chosen) = select_due_slot(lp) else {
            continue;
        };
        let idx = (chosen % RING_CAP as u64) as usize;
        // SAFETY: `chosen` is in `read_pos..write_pos` (observed with `Acquire`
        // in `select_due_slot`), so `slots[idx]` is immutable until `read_pos`
        // is updated with `Release` below.
        let uploaded = unsafe {
            let slot = &*lp.ring.slots[idx].get();
            upload_aligned(
                &images,
                &device,
                &queue,
                &mut pool,
                &lp.planes[..3],
                &slot.planes,
            )
        };
        if uploaded {
            lp.ring.read_pos.store(chosen + 1, Ordering::Release);
            if first_done {
                lp.showing.store(true, Ordering::Release);
            }
            if let Some(worker) = lp.worker.get() {
                worker.unpark();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped files are 1280 × 720 (16:9), exact 60/1 fps (`370` & `480`
    /// samples, zero ms truncation), 8-bit limited-range BT.709, opaque, and
    /// repeat cleanly with 256-byte row-aligned plane strides.
    #[test]
    fn the_shipped_loops_are_what_the_shader_converts() {
        for (name, bytes, expected_samples, expected_ns) in [
            ("world", WORLD, 370usize, 6_166_666_666u64),
            ("tavern", TAVERN, 480usize, 8_000_000_000u64),
        ] {
            let sequence = Sequence::read(bytes, &Bounds::SHIPPED_LOOP)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let track = &sequence.colour;
            assert_eq!((track.width, track.height), (1280, 720), "{name}");
            assert_eq!(track.samples.len(), expected_samples, "{name}");
            assert_eq!(track.duration_ns(), expected_ns, "{name} exact 60/1 fps");
            assert!(sequence.repeats, "{name} repeats");
            assert!(sequence.alpha.is_none(), "{name} is opaque");

            let mut player = Player::open_static(bytes, &Bounds::SHIPPED_LOOP, DECODE_THREADS)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let mut planes = AlignedPlanes::with_dimensions(track.width, track.height);
            let index = player
                .advance_aligned(&mut planes)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(index, 0);
            assert_eq!((planes.width, planes.height), (1280, 720));
            assert_eq!((planes.y_stride, planes.c_stride), (1280, 768));
            assert!(matches!(planes.matrix, Matrix::Bt709), "{name}");
            assert!(!planes.full_range, "{name}");
        }
    }

    /// Constant `1.0×` playback tempo: stepping `t = 0.0..=10.0s` at 30 fps,
    /// 60 fps, and 120 fps produces identical wall-clock timestamps and
    /// selects the exact same `(pass, sample_index)` at every shared instant.
    #[test]
    fn playback_tempo_is_identical_across_thirty_sixty_and_one_twenty_fps() {
        for (name, bytes) in [("world", WORLD), ("tavern", TAVERN)] {
            let seq = Sequence::read(bytes, &Bounds::SHIPPED_LOOP).expect(name);
            let track = &seq.colour;

            let simulate = |fps: u64| -> Vec<(u64, (u64, usize))> {
                let mut clock = VistaClock::default();
                let total_steps = 10 * fps;
                let mut out = Vec::with_capacity(usize::try_from(total_steps + 1).unwrap());
                let mut prev_ns = 0u64;
                out.push((0, track.sample_at_ns(0, seq.repeats)));
                for step in 1..=total_steps {
                    let target_ns = step * 1_000_000_000 / fps;
                    clock.advance(target_ns - prev_ns, false, false);
                    prev_ns = target_ns;
                    out.push((clock.nanos, track.sample_at_ns(clock.nanos, seq.repeats)));
                }
                out
            };

            let at_30 = simulate(30);
            let at_60 = simulate(60);
            let at_120 = simulate(120);

            for (i, sample_30) in at_30.iter().enumerate() {
                assert_eq!(*sample_30, at_60[i * 2], "{name} 30 vs 60 at step {i}");
                assert_eq!(*sample_30, at_120[i * 4], "{name} 30 vs 120 at step {i}");
            }
            for (i, sample_60) in at_60.iter().enumerate() {
                assert_eq!(*sample_60, at_120[i * 2], "{name} 60 vs 120 at step {i}");
            }
            // At exactly t = 10.0s: 600 frames elapsed (`pass 1, sample 230` for
            // 370-frame world; `pass 1, sample 120` for 480-frame tavern).
            let expected_10s = if name == "world" { (1, 230) } else { (1, 120) };
            assert_eq!(at_60.last().unwrap().1, expected_10s, "{name} 1.0x at 10s");
        }
    }

    const PATIENCE: Duration = Duration::from_secs(30);

    /// Unseen loops stay parked; active loops fill the 6-slot lookahead ring
    /// and `first` frame without allocations, park while `paused` (minimised /
    /// occluded), reset `showing = false` when evicted after `keep`, and
    /// immediately unpark + refill when `select_due_slot` drains a full ring of
    /// stale-generation frames.
    #[test]
    fn a_loop_fills_its_lookahead_ring_pauses_when_hidden_and_resets_after_keep() {
        let mut images = Assets::<Image>::default();
        let lp = Loop::start_with_keep(false, WORLD, &mut images, Duration::from_millis(25))
            .expect("world loop starts");
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            lp.ring.write_pos.load(Ordering::Acquire),
            0,
            "parked unseen"
        );

        lp.active.store(true, Ordering::Release);
        lp.worker.get().unwrap().unpark();
        let deadline = Instant::now() + PATIENCE;
        while lp.ring.write_pos.load(Ordering::Acquire) < RING_CAP as u64
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            lp.ring.write_pos.load(Ordering::Acquire),
            RING_CAP as u64,
            "worker fills all 6 lookahead slots and parks"
        );
        assert_eq!(lp.ring.first_state.load(Ordering::Acquire), FIRST_READY);

        // Simulate paused (minimised/occluded) + freeing 2 slots: stays parked!
        lp.paused.store(true, Ordering::Release);
        lp.ring.read_pos.store(2, Ordering::Release);
        lp.worker.get().unwrap().unpark();
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            lp.ring.write_pos.load(Ordering::Acquire),
            RING_CAP as u64,
            "paused worker decodes 0 frames while hidden"
        );

        // Unpause: worker immediately refills the 2 freed lookahead slots
        // (`write_pos == 8, read_pos == 2`, so the 6-slot ring is 100% full).
        lp.paused.store(false, Ordering::Release);
        lp.worker.get().unwrap().unpark();
        let deadline = Instant::now() + PATIENCE;
        while lp.ring.write_pos.load(Ordering::Acquire) < (RING_CAP as u64 + 2)
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            lp.ring.write_pos.load(Ordering::Acquire),
            RING_CAP as u64 + 2
        );

        // Leave unseen past `keep` (`25 ms`) while the ring is full: worker
        // evicts its decoder, resets `showing = false`, and bumps `generation`.
        lp.showing.store(true, Ordering::Release);
        let gen_before = lp.generation.load(Ordering::Acquire);
        lp.active.store(false, Ordering::Release);
        lp.worker.get().unwrap().unpark();
        let deadline = Instant::now() + PATIENCE;
        while lp.generation.load(Ordering::Acquire) == gen_before && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            !lp.showing(),
            "showing reset so stale GPU frames never flash"
        );
        assert_eq!(lp.generation.load(Ordering::Acquire), gen_before + 1);

        // Pre-warm (`warm = true`, `visible = false`) on a full stale ring:
        // worker opens `Player` and parks on full ring; `select_due_slot` drains
        // all 6 stale-generation slots AND immediately unparks the worker so it
        // refills 6 fresh generation-2 frames without waiting for `NAP` (250 ms).
        lp.warm.store(true, Ordering::Release);
        lp.worker.get().unwrap().unpark();
        std::thread::sleep(Duration::from_millis(10));
        let drain_start = Instant::now();
        assert_eq!(select_due_slot(&lp), None);
        assert_eq!(
            lp.ring.read_pos.load(Ordering::Acquire),
            RING_CAP as u64 + 2,
            "all 6 stale-generation slots drained"
        );
        let target_write = (RING_CAP as u64 * 2) + 2;
        while lp.ring.write_pos.load(Ordering::Acquire) < target_write
            && drain_start.elapsed() < PATIENCE
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(lp.ring.write_pos.load(Ordering::Acquire), target_write);
        assert!(
            drain_start.elapsed() < NAP,
            "stale-ring drain unparked worker immediately ({:?} < {NAP:?})",
            drain_start.elapsed()
        );
        assert_eq!(select_due_slot(&lp), Some(RING_CAP as u64 + 2));
    }
}
