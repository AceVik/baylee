//! The front door's two moving pictures: the world behind the sign-in and
//! the tavern behind the lobby.
//!
//! Each is an AVIF image sequence of our own (`loops/world.avif`,
//! `loops/tavern.avif`: 1280 × 720, 30 frames a second, about a megabyte;
//! `art/baylee/README.md` has their provenance and the command that made
//! them), shipped inside the binary and decoded by `baylee-avif`. Until
//! 12.11.2026 the client piped 3840 × 2160 RGBA out of a player's own
//! `ffmpeg`, which no player has: 33 MB a frame, 1.5 GB of memory, a loop
//! that ran at the renderer's rate and a lobby pinned at 60 frames a second
//! (`docs/perf-client.md`).
//!
//! How a loop plays:
//!
//! - **By the scene's clock.** [`step`] tells each loop how far the scene's
//!   own clock has moved (`VistaParams::portal.z`, which stands still while a
//!   menu rests, under `reduce_motion` and at ambient `Low`), and the worker
//!   decodes up to the frame that belongs there. A renderer at 15 frames a
//!   second shows every second frame at the loop's true speed; one at rest
//!   decodes nothing. A menu whose loop is seen does not come to rest
//!   (`super::Moving`, `graphics::Showing::picture`): left alone it draws
//!   at the idle rate and the film runs on.
//! - **Off the frame.** One parked thread per loop decodes into planes of its
//!   own; the render world copies the newest three planes into three `R8`
//!   textures (`RenderQueue::write_texture`, 1.4 MB a frame), and
//!   `vista.wesl` turns them into colour. No `Assets<Image>` is written and
//!   no texture is made after start-up.
//! - **Sharp where nothing moves.** The loop is a third of the still's
//!   width. The shader keeps the loop's first frame beside the running one
//!   and, wherever the two agree, draws the 3840 × 2160 still's own detail
//!   over the loop (`vista.wesl`, `moving`).
//! - **Faded in.** A surface shows its still until the first frame is on the
//!   GPU ([`Loop::showing`]) and [`light`] then brings the loop in over
//!   [`FADE_IN`] seconds, so nothing pops.
//! - **Given back.** A loop nobody sees parks at once and drops its decoder
//!   after [`KEEP`].
//!
//! The files are ours and are read under `Bounds::SHIPPED_LOOP`; no file
//! from outside reaches this decoder (`baylee-avif`'s own docs say why).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use baylee_avif::{Bounds, Frame, Matrix, Player, Sequence};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Extent3d, TexelCopyBufferLayout, TextureDimension, TextureFormat,
};
use bevy::render::renderer::RenderQueue;
use bevy::render::texture::GpuImage;

use super::{Vista, VistaMaterial};

/// The world behind the sign-in.
const WORLD: &[u8] = include_bytes!("loops/world.avif");
/// The tavern behind the lobby.
const TAVERN: &[u8] = include_bytes!("loops/tavern.avif");

/// How long a loop takes to come in over its still.
const FADE_IN: f32 = 0.6;

/// How long a loop nobody sees keeps its decoder.
const KEEP: Duration = Duration::from_secs(4);

/// The most frames decoded for one reading of the clock. A renderer slower
/// than the loop skips frames; one that stalls (a menu resting at a frame a
/// second) is not wound forward: the loop goes on from the frame it got to.
const CATCH_UP: usize = 3;

/// How long a worker whose frame is the clock's sleeps before it looks
/// again. [`step`] wakes it when the clock moves; this is only the net under
/// a wake-up that was lost.
const NAP: Duration = Duration::from_millis(250);

/// A loop's planes on the GPU: the running frame's Y, U and V, then the
/// first frame's.
pub(crate) type Planes = [Handle<Image>; 6];

/// One loop: its textures, its clock and its worker.
pub(crate) struct Loop {
    /// Whether this is the tavern's (else the world's).
    tavern: bool,
    planes: Planes,
    size: UVec2,
    /// When each frame begins, in milliseconds from the loop's start.
    starts: Vec<u32>,
    /// How long one pass takes.
    total_ms: u32,
    /// How far the scene's clock has moved, in milliseconds.
    wanted_ms: AtomicU64,
    /// Somebody sees the loop.
    active: AtomicBool,
    stop: AtomicBool,
    /// The first frame and a running frame are on the GPU.
    showing: AtomicBool,
    /// The decoder refused the file: the still stands for the session.
    failed: AtomicBool,
    ready: Mutex<Ready>,
    worker: OnceLock<std::thread::Thread>,
}

/// What the worker hands the render world.
#[derive(Default)]
struct Ready {
    /// Counts the frames handed over.
    seq: u64,
    /// The count the render world has copied.
    copied: u64,
    frame: Frame,
    /// The first frame, until the render world has copied it.
    first: Option<Frame>,
    first_copied: bool,
}

impl Drop for Loop {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.get() {
            worker.unpark();
        }
    }
}

/// The frame shown `ms` into a loop whose frames begin at `starts` and
/// whose pass takes `total_ms`.
fn frame_at(starts: &[u32], total_ms: u32, ms: u64) -> usize {
    if total_ms == 0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation)] // below `total_ms`, a u32
    let at = (ms % u64::from(total_ms)) as u32;
    starts
        .partition_point(|start| *start <= at)
        .saturating_sub(1)
}

/// What to add to a clock reading `clock` so that the loop's time is
/// `loop_ms` there: `frame_at(.., clock + shift)` is then the frame that
/// begins at `loop_ms`.
fn shift_to(loop_ms: u32, clock: u64, total_ms: u32) -> u64 {
    if total_ms == 0 {
        return 0;
    }
    let total = u64::from(total_ms);
    (u64::from(loop_ms) % total + total - clock % total) % total
}

impl Loop {
    /// Whether a surface of this kind draws this loop.
    fn plays(&self, kind: Vista) -> bool {
        self.tavern == (kind == Vista::Interior)
    }

    /// Whether the loop is on the GPU and may be drawn.
    pub(crate) fn showing(&self) -> bool {
        self.showing.load(Ordering::Relaxed)
    }

    /// Reads a shipped file's tables, makes its textures and starts its
    /// worker, parked. `None` if the file is not what the shader converts.
    fn start(tavern: bool, bytes: &'static [u8], images: &mut Assets<Image>) -> Option<Arc<Self>> {
        let sequence = Sequence::read(bytes, &Bounds::SHIPPED_LOOP).ok()?;
        let track = &sequence.colour;
        let size = UVec2::new(track.width, track.height);
        let chroma = (size + UVec2::ONE) / 2;
        let mut starts = Vec::with_capacity(track.samples.len());
        let mut at = 0u32;
        for sample in &track.samples {
            starts.push(at);
            at = at.saturating_add(sample.duration_ms);
        }
        let mut plane = |size: UVec2, fill: u8| {
            images.add(Image::new_fill(
                Extent3d {
                    width: size.x,
                    height: size.y,
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
            plane(size, 16),
            plane(chroma, 128),
            plane(chroma, 128),
            plane(size, 16),
            plane(chroma, 128),
            plane(chroma, 128),
        ];
        let lp = Arc::new(Self {
            tavern,
            planes,
            size,
            starts,
            total_ms: at,
            wanted_ms: AtomicU64::new(0),
            active: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            showing: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            ready: Mutex::new(Ready::default()),
            worker: OnceLock::new(),
        });
        let theirs = Arc::downgrade(&lp);
        let worker = std::thread::Builder::new()
            .name(if tavern { "tavern-loop" } else { "world-loop" }.to_string())
            .spawn(move || work(&theirs, bytes))
            .ok()?;
        let _ = lp.worker.set(worker.thread().clone());
        Some(lp)
    }

    /// Gives the loop up for the session: its still stands.
    fn fail(&self, why: &dyn std::fmt::Display) {
        if !self.failed.swap(true, Ordering::Relaxed) {
            warn!(
                "vista: the {} loop does not play: {why}",
                if self.tavern { "tavern" } else { "world" }
            );
        }
    }
}

/// The worker: parked until somebody sees the loop, then decoding up to the
/// frame the scene's clock asks for.
fn work(lp: &std::sync::Weak<Loop>, bytes: &'static [u8]) {
    let mut player: Option<Player> = None;
    let mut frame = Frame::default();
    let mut shown: Option<usize> = None;
    let mut first_sent = false;
    // What is added to the clock's reading to get the loop's own time. It
    // is set when the decoder is opened, so the loop starts at its first
    // frame and is not wound forward to wherever the clock has got to, and
    // again whenever the clock ran further ahead than `CATCH_UP`.
    let mut shift = 0u64;
    let mut unseen_since: Option<Instant> = None;
    loop {
        // Held only for one turn, so the last owner's drop ends the thread.
        let Some(lp) = lp.upgrade() else {
            return;
        };
        if lp.stop.load(Ordering::Relaxed) {
            return;
        }
        if !lp.active.load(Ordering::Relaxed) || lp.failed.load(Ordering::Relaxed) {
            let since = *unseen_since.get_or_insert_with(Instant::now);
            let waited = since.elapsed();
            drop(lp);
            if waited >= KEEP {
                player = None;
                shown = None;
                std::thread::park();
            } else {
                std::thread::park_timeout(KEEP.saturating_sub(waited));
            }
            continue;
        }
        unseen_since = None;
        let wanted = lp.wanted_ms.load(Ordering::Relaxed);
        if player.is_none() {
            match Player::open(Arc::from(bytes), &Bounds::SHIPPED_LOOP, 1) {
                Ok(opened) => {
                    shift = shift_to(0, wanted, lp.total_ms);
                    player = Some(opened);
                }
                Err(e) => {
                    lp.fail(&e);
                    continue;
                }
            }
        }
        let Some(decoder) = player.as_mut() else {
            continue;
        };
        let target = frame_at(&lp.starts, lp.total_ms, wanted.wrapping_add(shift));
        if shown == Some(target) {
            drop(lp);
            std::thread::park_timeout(NAP);
            continue;
        }
        let mut decoded = None;
        for _ in 0..CATCH_UP {
            match decoder.advance(&mut frame) {
                Ok((index, _)) => {
                    let fits = frame.width == lp.size.x
                        && frame.height == lp.size.y
                        && matches!(frame.matrix, Matrix::Bt709)
                        && !frame.full_range;
                    if !fits {
                        lp.fail(&"a frame is not 8-bit limited-range BT.709 of the track's size");
                        break;
                    }
                    if index == 0
                        && !first_sent
                        && let Ok(mut ready) = lp.ready.lock()
                    {
                        ready.first = Some(frame.clone());
                        first_sent = true;
                    }
                    decoded = Some(index);
                    if index == target {
                        break;
                    }
                }
                Err(e) => {
                    lp.fail(&e);
                    break;
                }
            }
        }
        if let Some(index) = decoded
            && let Ok(mut ready) = lp.ready.lock()
        {
            std::mem::swap(&mut frame, &mut ready.frame);
            ready.seq = ready.seq.wrapping_add(1);
            shown = Some(index);
            if index != target {
                // The clock is further on than one go reaches: the loop's
                // time is now this frame's, and runs on from here.
                let start = lp.starts.get(index).copied().unwrap_or(0);
                shift = shift_to(start, wanted, lp.total_ms);
            }
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

/// Starts both loops, parked, and has the render world copy their frames.
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

/// Tells each loop whether anybody sees it and how far the scene's clock
/// has moved, and wakes its worker when either changed.
fn step(
    loops: Res<Loops>,
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
    let mut any = false;
    for lp in loops.0.iter() {
        let wanted = surfaces.iter().find_map(|(kind, handle, own, inherited)| {
            // `paint` hides a surface that is not shown; a hidden screen
            // hides it from above.
            if !lp.plays(*kind) || *own == Visibility::Hidden || !inherited.get() {
                return None;
            }
            let params = &materials.get(&handle.0)?.params;
            // Energy, and the scene's own alpha: a frozen or an unseen
            // scene asks for no frame.
            (params.view.w > 0.0 && params.air.z > 0.0).then(|| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let ms = (f64::from(params.portal.z.max(0.0)) * 1000.0) as u64;
                ms
            })
        });
        let active = wanted.is_some();
        any |= active && !lp.failed.load(Ordering::Relaxed);
        let mut wake = lp.active.swap(active, Ordering::Relaxed) != active;
        if let Some(ms) = wanted {
            wake |= lp.wanted_ms.swap(ms, Ordering::Relaxed) != ms;
        }
        if wake && let Some(worker) = lp.worker.get() {
            worker.unpark();
        }
    }
    // A film that is seen keeps its menu drawing (`quality::pace`).
    moving.set_if_neq(super::Moving(any));
}

/// Hands each surface its loop's planes, once, and brings the loop in over
/// the still when its first frame is on the GPU. `paint` keeps
/// `VistaParams::moving` as it finds it; this is its one writer.
fn light(
    time: Res<Time>,
    loops: Res<Loops>,
    surfaces: Query<(&Vista, &MaterialNode<VistaMaterial>)>,
    materials: Option<ResMut<Assets<VistaMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let step = time.delta_secs() / FADE_IN;
    for (kind, handle) in &surfaces {
        let Some(lp) = loops.of(*kind) else {
            continue;
        };
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        let now = material.params.moving.x;
        let next = super::towards(now, if lp.showing() { 1.0 } else { 0.0 }, step);
        let wired = material
            .planes()
            .zip(&lp.planes)
            .all(|(a, b)| a.id() == b.id());
        // A write is a new bind group or a new uniform: only on a change.
        #[allow(clippy::float_cmp)] // `towards` ends exactly on its target
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

/// Copies one frame's three planes into three textures. `false`, and
/// nothing written, while a texture is not on the GPU yet.
fn write(
    images: &RenderAssets<GpuImage>,
    queue: &RenderQueue,
    planes: &[Handle<Image>],
    frame: &Frame,
) -> bool {
    let chroma = (frame.chroma_width(), frame.chroma_height());
    let parts = [
        (&planes[0], &frame.y, (frame.width, frame.height)),
        (&planes[1], &frame.u, chroma),
        (&planes[2], &frame.v, chroma),
    ];
    let fits = parts.iter().all(|(handle, bytes, (width, height))| {
        images.get(handle.id()).is_some_and(|gpu| {
            let size = gpu.texture_descriptor.size;
            size.width == *width
                && size.height == *height
                && gpu.texture_descriptor.format == TextureFormat::R8Unorm
                && bytes.len() == (*width as usize) * (*height as usize)
        })
    });
    if !fits {
        return false;
    }
    for (handle, bytes, (width, height)) in parts {
        let Some(gpu) = images.get(handle.id()) else {
            return false;
        };
        queue.write_texture(
            gpu.texture.as_image_copy(),
            bytes,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: None,
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
    true
}

/// The render world's half: the first frame once, then the newest frame
/// whenever there is one.
fn copy(loops: Option<Res<Loops>>, images: Res<RenderAssets<GpuImage>>, queue: Res<RenderQueue>) {
    let Some(loops) = loops else {
        return;
    };
    for lp in loops.0.iter() {
        if !lp.active.load(Ordering::Relaxed) {
            continue;
        }
        let Ok(mut ready) = lp.ready.try_lock() else {
            continue;
        };
        if let Some(first) = ready.first.take() {
            if write(&images, &queue, &lp.planes[3..], &first) {
                ready.first_copied = true;
            } else {
                ready.first = Some(first);
            }
        }
        if ready.seq != ready.copied && write(&images, &queue, &lp.planes[..3], &ready.frame) {
            ready.copied = ready.seq;
            if ready.first_copied {
                lp.showing.store(true, Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame is shown from its start to the next one's, the loop wraps at
    /// its end, and a clock that never moved shows the first frame.
    #[test]
    fn the_clock_picks_the_frame_and_wraps_at_the_loops_end() {
        let starts = [0, 33, 66, 100];
        let total = 133;
        assert_eq!(frame_at(&starts, total, 0), 0);
        assert_eq!(frame_at(&starts, total, 32), 0);
        assert_eq!(frame_at(&starts, total, 33), 1);
        assert_eq!(frame_at(&starts, total, 99), 2);
        assert_eq!(frame_at(&starts, total, 132), 3);
        assert_eq!(frame_at(&starts, total, 133), 0);
        assert_eq!(frame_at(&starts, total, 133 * 7 + 70), 2);
        assert_eq!(frame_at(&[], 0, 5), 0);
    }

    /// The loop's time is set against the clock: at its first frame when
    /// the decoder opens, wherever the clock stands, and at the frame it got
    /// to when the clock ran ahead; from there it runs at the clock's speed.
    #[test]
    fn the_loops_time_is_set_against_the_clock_and_then_runs_with_it() {
        let starts = [0, 33, 66, 100];
        let total = 133;
        for clock in [0u64, 1, 132, 133, 5_000_017, u64::from(u32::MAX) * 9] {
            let shift = shift_to(0, clock, total);
            assert!(shift < u64::from(total));
            assert_eq!(frame_at(&starts, total, clock + shift), 0, "{clock}");
            assert_eq!(frame_at(&starts, total, clock + 40 + shift), 1, "{clock}");
            let shift = shift_to(66, clock, total);
            assert_eq!(frame_at(&starts, total, clock + shift), 2, "{clock}");
            assert_eq!(frame_at(&starts, total, clock + 34 + shift), 3, "{clock}");
            assert_eq!(frame_at(&starts, total, clock + 67 + shift), 0, "{clock}");
        }
        assert_eq!(shift_to(7, 9, 0), 0);
    }

    /// The shipped files are what the worker accepts and `vista.wesl`
    /// converts: 16:9, within the shipped bounds, 8-bit limited-range BT.709,
    /// and they repeat. A loop re-encoded another way fails here and not as
    /// a still that never moves.
    #[test]
    fn the_shipped_loops_are_what_the_shader_converts() {
        for (name, bytes) in [("world", WORLD), ("tavern", TAVERN)] {
            let sequence = Sequence::read(bytes, &Bounds::SHIPPED_LOOP)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let track = &sequence.colour;
            assert_eq!(track.width * 9, track.height * 16, "{name} is 16:9");
            assert!(track.samples.len() > 30, "{name} moves");
            assert!(sequence.alpha.is_none(), "{name} is opaque");
            let mut player = Player::open(Arc::from(bytes), &Bounds::SHIPPED_LOOP, 1)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let mut frame = Frame::default();
            let (index, shown_ms) = player
                .advance(&mut frame)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(index, 0);
            assert!(shown_ms >= 16, "{name}: no faster than 60 frames a second");
            assert_eq!((frame.width, frame.height), (track.width, track.height));
            assert!(matches!(frame.matrix, Matrix::Bt709), "{name}");
            assert!(!frame.full_range, "{name}");
            assert_eq!(frame.y.len(), (track.width * track.height) as usize);
        }
    }

    /// How long a test waits for a frame before it calls the loop dead. It
    /// is the net under a worker that never hands over, not a measurement:
    /// at 20 s it was one, and a loaded machine failed it.
    const PATIENCE: Duration = Duration::from_secs(120);

    /// A loop nobody sees decodes nothing, and one that is seen hands over
    /// its first frame and then frames by the clock.
    #[test]
    fn a_loop_decodes_only_while_it_is_seen() {
        let mut images = Assets::<Image>::default();
        let lp = Loop::start(false, WORLD, &mut images).expect("the world loop starts");
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(lp.ready.lock().unwrap().seq, 0, "parked while unseen");

        lp.active.store(true, Ordering::Relaxed);
        lp.wanted_ms.store(1, Ordering::Relaxed);
        lp.worker.get().unwrap().unpark();
        let handed = |lp: &Loop| lp.ready.lock().unwrap().seq;
        let deadline = Instant::now() + PATIENCE;
        while handed(&lp) == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        {
            let ready = lp.ready.lock().unwrap();
            assert_eq!(ready.seq, 1, "one frame for one reading of the clock");
            assert!(
                ready.first.is_some(),
                "the first frame is kept for the shader"
            );
            assert_eq!(ready.frame.width, lp.size.x);
        }
        // The clock stands: no further frame.
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(handed(&lp), 1);
        // The clock moves on by three frames: one more hand-over.
        lp.wanted_ms.store(1 + 100, Ordering::Relaxed);
        lp.worker.get().unwrap().unpark();
        let deadline = Instant::now() + PATIENCE;
        while handed(&lp) == 1 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(handed(&lp), 2);
    }
}
