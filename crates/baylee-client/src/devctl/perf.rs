//! What a frame costs, read from outside: `/perf`, `/hide` and `/msaa`.
//!
//! `docs/perf-client.md` holds the numbers these routes produced and how
//! they were taken. Three things are measured in here and the rest from
//! outside the process, because the rest cannot be read honestly from inside
//! it on this hardware:
//!
//! - **Frame time** is `Time<Real>`'s delta, kept per frame, so a caller gets
//!   a median and a tail rather than an average a single hitch hides in.
//! - **Main-schedule time** is the wall time from `First` to `Last`: what the
//!   main thread spends in the app's own systems. Rendering runs pipelined
//!   on its own thread and is not in it.
//! - **Allocations per frame** come from [`CountingAlloc`], which `main.rs`
//!   installs as the global allocator in a `dev-control` build only.
//!
//! GPU time is not here. Bevy's `RenderDiagnosticsPlugin` records only CPU
//! time on Metal (its own documentation says so), so a number from it would
//! be a CPU number under a GPU name. GPU time comes from `xctrace`'s Metal
//! System Trace and the process's energy from `top`, both outside.
//!
//! `/hide` takes a whole kind of drawing away — every entity wearing one
//! material — so its cost can be read as the difference it makes, and its
//! *contribution* as the difference a full-frame diff of two screenshots
//! makes. Both halves of "is this effect worth it" come from one switch.

use bevy::camera::Camera;
use bevy::ecs::entity::Entities;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Allocations made since the process started.
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
/// Bytes asked for since the process started (a `realloc` counts its new
/// size, which is what a growing `Vec` really asks the allocator for).
static ALLOCATED: AtomicU64 = AtomicU64::new(0);

/// The system allocator, counting.
///
/// Two relaxed atomic adds per call: cheap enough that the frame times it is
/// read beside are not its own, and only ever in a `dev-control` build. While
/// `/allocs` samples, every n-th allocation also keeps its call stack
/// ([`sample`]).
pub struct CountingAlloc;

/// Every how many allocations `/allocs` keeps a call stack; `0` = not
/// sampling.
static SAMPLE_EVERY: AtomicU64 = AtomicU64::new(0);

/// At most this many stacks are kept per sampling window: resolving them is
/// what costs, and a few thousand already rank the sites.
const MAX_SAMPLES: usize = 4000;

/// The stacks kept, with the thread each came from.
static SAMPLES: std::sync::Mutex<Vec<(String, std::backtrace::Backtrace)>> =
    std::sync::Mutex::new(Vec::new());

std::thread_local! {
    /// This thread is inside [`sample`]: what it allocates there is the
    /// sampler's own and is neither counted nor sampled.
    static SAMPLING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// This thread's allocations since sampling began, for the every-n-th.
    static TICK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// This is the thread the main world's schedules run on (set by the
    /// probe's first system), whose allocations `/perf` also counts alone.
    static MAIN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Allocations made on the main world's thread since the process started.
static MAIN_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// Counts one allocation of `bytes`, and keeps its stack if it is the n-th
/// while `/allocs` samples. The sampler's own allocations are not counted.
fn count(bytes: usize) {
    let every = SAMPLE_EVERY.load(Ordering::Relaxed);
    if every != 0 {
        let inside = SAMPLING.try_with(std::cell::Cell::get).unwrap_or(true);
        if inside {
            return;
        }
        let tick = TICK.try_with(|t| {
            let n = t.get() + 1;
            t.set(n);
            n
        });
        if tick.is_ok_and(|n| n % every == 0) {
            sample();
        }
    }
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    ALLOCATED.fetch_add(bytes as u64, Ordering::Relaxed);
    if MAIN.try_with(std::cell::Cell::get).unwrap_or(false) {
        MAIN_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    }
}

/// Keeps the current call stack, unresolved (resolving waits for `/allocs`).
#[cold]
fn sample() {
    let _ = SAMPLING.try_with(|s| s.set(true));
    if let Ok(mut kept) = SAMPLES.lock()
        && kept.len() < MAX_SAMPLES
    {
        let thread = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        kept.push((thread, std::backtrace::Backtrace::force_capture()));
    }
    let _ = SAMPLING.try_with(|s| s.set(false));
}

// SAFETY: every method forwards its arguments unchanged to `System`, which
// upholds `GlobalAlloc`'s contract; the counters and the sampler never touch
// the allocation being made, and the returned pointer is `System`'s own. The
// sampler allocates itself, which re-enters this allocator: the thread-local
// `SAMPLING` flag makes that inner call a plain forward to `System`, so it
// cannot recurse, and `try_with` keeps a thread being torn down out of it.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: the caller's `layout` obligations pass through unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: as `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System` through this allocator with this
        // `layout`, which is the caller's obligation.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count(new_size);
        // SAFETY: as `dealloc` for `ptr` and `layout`; `new_size` is the
        // caller's to keep valid.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// The counters as one reading: every thread's allocations and bytes, and
/// the main world's thread's allocations.
fn allocations() -> (u64, u64, u64) {
    (
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED.load(Ordering::Relaxed),
        MAIN_ALLOCATIONS.load(Ordering::Relaxed),
    )
}

/// The window being measured: everything since the last reset.
#[derive(Resource)]
pub(super) struct Probe {
    since: Instant,
    frame_ms: Vec<f32>,
    update_ms: Vec<f32>,
    update_began: Option<Instant>,
    allocations_at_reset: (u64, u64, u64),
    /// System count per schedule, taken once every schedule exists.
    systems: usize,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            since: Instant::now(),
            frame_ms: Vec::with_capacity(4096),
            update_ms: Vec::with_capacity(4096),
            update_began: None,
            allocations_at_reset: allocations(),
            systems: 0,
        }
    }
}

/// Which kinds `/hide` has taken away.
#[derive(Resource, Default)]
pub(super) struct Hidden(BTreeSet<String>);

/// Installs the probe's systems and the switches.
pub(super) fn install(app: &mut App) {
    use crate::{
        ambience, arrowmat, badgemat, cardmat, feltmat, floormat, frontal, marksmat, matmat,
        platemat, shellmat, shellui, sky, vista,
    };
    app.init_resource::<Probe>()
        .init_resource::<Hidden>()
        .add_systems(First, begin_update)
        .add_systems(Last, end_update)
        .add_systems(PostStartup, count_systems)
        .add_systems(
            PostUpdate,
            (
                hide::<MeshMaterial3d<feltmat::FeltMaterial>>("felt"),
                hide::<MeshMaterial3d<matmat::MatMaterial>>("mat"),
                hide::<MeshMaterial3d<cardmat::CardMaterial>>("card"),
                hide::<MeshMaterial3d<floormat::FloorMaterial>>("floor"),
                hide::<MeshMaterial3d<shellmat::ShellMaterial>>("shell"),
                hide::<MeshMaterial3d<platemat::PlateMaterial>>("plate"),
                hide::<MeshMaterial3d<badgemat::BadgeMaterial>>("badge"),
                hide::<MeshMaterial3d<marksmat::MarksMaterial>>("marks"),
                hide::<MeshMaterial3d<arrowmat::ArrowMaterial>>("arrow"),
                hide::<MeshMaterial3d<sky::SkyMaterial>>("sky"),
                hide::<MaterialNode<vista::VistaMaterial>>("vista"),
                hide::<MaterialNode<ambience::AmbienceMaterial>>("ambience"),
                hide::<MaterialNode<frontal::FrontalMaterial>>("frontal"),
                hide::<MaterialNode<cardmat::CardUiMaterial>>("card_ui"),
                hide::<MaterialNode<shellui::ShellUiMaterial>>("shell_ui"),
            )
                .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
}

fn begin_update(mut probe: ResMut<Probe>) {
    let _ = MAIN.try_with(|main| main.set(true));
    probe.update_began = Some(Instant::now());
}

fn end_update(mut probe: ResMut<Probe>, time: Res<Time<Real>>) {
    if let Some(began) = probe.update_began.take() {
        let ms = began.elapsed().as_secs_f32() * 1000.0;
        probe.update_ms.push(ms);
    }
    let frame = time.delta_secs() * 1000.0;
    if frame > 0.0 {
        probe.frame_ms.push(frame);
    }
}

/// Counted once, after startup: while a schedule runs it is taken out of
/// `Schedules`, so counting from inside the frame would miss the running one.
fn count_systems(mut probe: ResMut<Probe>, schedules: Res<Schedules>) {
    probe.systems = schedules
        .iter()
        .map(|(_, schedule)| schedule.systems_len())
        .sum();
}

/// A percentile of an unsorted sample; `0.0` for an empty one.
fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let at = ((sorted.len() - 1) as f32 * p).round() as usize;
    sorted[at.min(sorted.len() - 1)]
}

/// One line of `{p50, p95, p99, max, mean}` for a sample.
fn spread(sample: &[f32]) -> String {
    let mut sorted = sample.to_vec();
    sorted.sort_by(f32::total_cmp);
    #[allow(clippy::cast_precision_loss)]
    let mean = if sorted.is_empty() {
        0.0
    } else {
        sorted.iter().sum::<f32>() / sorted.len() as f32
    };
    format!(
        "{{\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"max\":{:.3},\"mean\":{:.3}}}",
        percentile(&sorted, 0.50),
        percentile(&sorted, 0.95),
        percentile(&sorted, 0.99),
        sorted.last().copied().unwrap_or(0.0),
        mean,
    )
}

/// `/perf`: the window so far, then a fresh one when the body asks
/// (`{"reset":true}`), so a caller measures one interval per two calls.
pub(super) fn answer(probe: &mut Probe, entities: &Entities, reset: bool) -> String {
    let secs = probe.since.elapsed().as_secs_f64();
    let frames = probe.frame_ms.len() as u64;
    let (count, bytes, main) = allocations();
    let (count0, bytes0, main0) = probe.allocations_at_reset;
    let per_frame = |total: u64| {
        #[allow(clippy::cast_precision_loss)]
        let per = if frames == 0 {
            0.0
        } else {
            total as f64 / frames as f64
        };
        per
    };
    #[allow(clippy::cast_precision_loss)]
    let fps = if secs > 0.0 {
        frames as f64 / secs
    } else {
        0.0
    };
    let body = format!(
        "{{\"ok\":true,\"secs\":{secs:.3},\"frames\":{frames},\"fps\":{fps:.2},\
         \"frame_ms\":{},\"update_ms\":{},\"entities\":{},\"systems\":{},\
         \"allocs_per_frame\":{:.1},\"alloc_bytes_per_frame\":{:.0},\
         \"main_allocs_per_frame\":{:.1}}}",
        spread(&probe.frame_ms),
        spread(&probe.update_ms),
        entities.count_spawned(),
        probe.systems,
        per_frame(count - count0),
        per_frame(bytes - bytes0),
        per_frame(main - main0),
    );
    if reset {
        let systems = probe.systems;
        *probe = Probe {
            systems,
            ..Probe::default()
        };
    }
    body
}

/// `/allocs {"every":N}`: starts keeping the call stack of every N-th
/// allocation on each thread (at most [`MAX_SAMPLES`]); `/allocs {}` stops
/// and answers with where they came from, as plain text: per thread, the
/// first frame in this workspace's code, and below each such site the first
/// frame outside the allocator and the standard library. What allocates at
/// rest is read here, then removed at its site.
pub(super) fn allocs(every: Option<u64>) -> String {
    use std::fmt::Write as _;
    if let Some(every) = every.filter(|n| *n > 0) {
        if let Ok(mut kept) = SAMPLES.lock() {
            kept.clear();
        }
        SAMPLE_EVERY.store(every, Ordering::Relaxed);
        return format!("{{\"ok\":true,\"every\":{every}}}");
    }
    SAMPLE_EVERY.store(0, Ordering::Relaxed);
    let kept = SAMPLES
        .lock()
        .map(|mut kept| std::mem::take(&mut *kept))
        .unwrap_or_default();
    let mut sites: std::collections::BTreeMap<(String, String, String), (usize, String)> =
        std::collections::BTreeMap::new();
    let mut threads: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (thread, trace) in &kept {
        let text = format!("{trace:#}");
        let (ours, first) = sites_of(&text);
        *threads.entry(thread.clone()).or_default() += 1;
        let entry = sites
            .entry((thread.clone(), ours, first))
            .or_insert_with(|| (0, text));
        entry.0 += 1;
    }
    let mut ranked: Vec<_> = sites.into_iter().collect();
    ranked.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    let mut out = format!("samples {}\n", kept.len());
    for (thread, n) in &threads {
        let _ = writeln!(out, "thread {n:6} {thread}");
    }
    for ((thread, ours, first), (n, _)) in ranked.iter().take(300) {
        let _ = writeln!(out, "{n:6} [{thread}] {ours}  <=  {first}");
    }
    // One whole stack for each of the commonest sites on the main thread:
    // the frame that names a site is often a library's, and the system that
    // called it is further down.
    for ((thread, ours, first), (n, text)) in ranked
        .iter()
        .filter(|((thread, _, _), _)| thread == "main")
        .take(40)
    {
        let _ = writeln!(out, "--- {n} [{thread}] {ours} <= {first}");
        for name in frame_names(text).take(60) {
            out.push_str("    ");
            out.push_str(&name[..name.len().min(160)]);
            out.push('\n');
        }
    }
    out
}

/// The function names of a resolved backtrace's text, innermost first, in
/// either of its formats (`  3: name` or, full, `  3:   0x1f00 - name`), the
/// `at file:line` lines left out.
fn frame_names(trace: &str) -> impl Iterator<Item = String> {
    trace
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("at "))
        .filter_map(|line| line.split_once(": ").map(|(_, name)| name.trim()))
        .map(|name| {
            let name = if name.starts_with("0x") {
                name.split_once(" - ").map_or(name, |(_, name)| name)
            } else {
                name
            };
            // The full format names a crate with its hash, `std[15f5…]::`.
            let mut plain = String::with_capacity(name.len());
            let mut rest = name;
            while let Some(open) = rest.find('[') {
                let close = rest[open..].find(']').map(|c| open + c);
                match close {
                    Some(close) if rest[open + 1..close].chars().all(|c| c.is_ascii_hexdigit()) => {
                        plain.push_str(&rest[..open]);
                        rest = &rest[close + 1..];
                    }
                    _ => {
                        plain.push_str(&rest[..=open]);
                        rest = &rest[open + 1..];
                    }
                }
            }
            plain.push_str(rest);
            plain
        })
}

/// The first frame in this workspace's crates and the first frame outside
/// the allocator, `std`, `core` and `alloc`, read off a resolved backtrace's
/// text.
fn sites_of(trace: &str) -> (String, String) {
    let frames: Vec<String> = frame_names(trace).collect();
    let plumbing = |name: &str| {
        [
            "std::",
            "core::",
            "alloc::",
            "<alloc::",
            "<core::",
            "<std::",
            "__rust",
            "baylee_client::devctl::perf",
            "<baylee_client::devctl::perf",
            "hashbrown::",
            "<hashbrown::",
            "_malloc",
            "malloc",
            "realloc",
        ]
        .iter()
        .any(|p| name.starts_with(p))
    };
    let ours = frames
        .iter()
        .find(|name| {
            (name.starts_with("baylee") || name.starts_with("<baylee"))
                && !name.contains("devctl::perf")
                && !name.contains("standalone::run")
                && !name.starts_with("baylee_client::main")
        })
        .map_or_else(|| "-".to_string(), Clone::clone);
    let first = frames
        .iter()
        .find(|name| !plumbing(name))
        .map_or_else(|| "-".to_string(), Clone::clone);
    (ours, first)
}

/// `/hide {"what":"felt","hidden":true}`: takes a kind of drawing away (or
/// gives it back), answering with every kind now hidden.
pub(super) fn set_hidden(hidden: &mut Hidden, what: &str, on: bool) -> String {
    if on {
        hidden.0.insert(what.to_string());
    } else {
        hidden.0.remove(what);
    }
    let list: Vec<String> = hidden.0.iter().map(|k| format!("\"{k}\"")).collect();
    format!("{{\"ok\":true,\"hidden\":[{}]}}", list.join(","))
}

/// `/msaa {"samples":1}`: every camera's multisampling, for measuring what it
/// costs. Answers with what was set.
pub(super) fn set_msaa(cameras: &mut Query<&mut Msaa, With<Camera>>, samples: u32) -> String {
    let msaa = match samples {
        1 => Msaa::Off,
        2 => Msaa::Sample2,
        4 => Msaa::Sample4,
        8 => Msaa::Sample8,
        _ => return "{\"error\":\"samples must be 1, 2, 4 or 8\"}".to_string(),
    };
    let mut set = 0;
    for mut camera in cameras.iter_mut() {
        *camera = msaa;
        set += 1;
    }
    format!("{{\"ok\":true,\"samples\":{samples},\"cameras\":{set}}}")
}

/// The entities one kind of drawing is made of.
type HiddenQuery<'w, 's, C> = Query<'w, 's, (Entity, &'static mut Visibility), With<C>>;
/// What each hidden entity's visibility was before it was hidden.
type Kept<'s> = Local<'s, HashMap<Entity, Visibility>>;

/// `/executor {"single":true}`: every main-world schedule but the one
/// running this on one thread, or (`false`) back on the task pool — to
/// measure what the multi-threaded executor's wake-ups cost a client whose
/// systems are mostly a few microseconds each.
pub(super) fn set_executor(schedules: &mut Schedules, single: bool) -> String {
    use bevy::ecs::schedule::{MultiThreadedExecutor, SingleThreadedExecutor};
    let mut set = 0;
    for (_, schedule) in schedules.iter_mut() {
        if single {
            schedule.set_executor(SingleThreadedExecutor::new());
        } else {
            schedule.set_executor(MultiThreadedExecutor::new());
        }
        set += 1;
    }
    format!("{{\"ok\":true,\"single\":{single},\"schedules\":{set}}}")
}

/// Keeps every entity wearing `C` hidden while its kind is asked to be, and
/// gives each back the visibility it had once it is not.
///
/// Re-applied every frame because the table sets visibility itself (a row's
/// scroll window hides and shows cards); what it last set is what comes back.
fn hide<C: Component>(kind: &'static str) -> impl FnMut(Res<Hidden>, HiddenQuery<C>, Kept) {
    move |hidden, mut drawn, mut kept| {
        if hidden.0.contains(kind) {
            for (entity, mut visibility) in &mut drawn {
                if *visibility != Visibility::Hidden {
                    kept.insert(entity, *visibility);
                    *visibility = Visibility::Hidden;
                }
            }
        } else if !kept.is_empty() {
            for (entity, was) in kept.drain() {
                if let Ok((_, mut visibility)) = drawn.get_mut(entity) {
                    *visibility = was;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percentile_reads_the_sorted_sample_at_its_rank() {
        let sorted = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&sorted, 0.5) - 3.0).abs() < f32::EPSILON);
        assert!((percentile(&sorted, 0.0) - 1.0).abs() < f32::EPSILON);
        assert!((percentile(&sorted, 1.0) - 5.0).abs() < f32::EPSILON);
        assert!(percentile(&[], 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn hiding_and_showing_a_kind_is_reported_and_undone() {
        let mut hidden = Hidden::default();
        assert_eq!(
            set_hidden(&mut hidden, "felt", true),
            "{\"ok\":true,\"hidden\":[\"felt\"]}"
        );
        set_hidden(&mut hidden, "vista", true);
        assert_eq!(
            set_hidden(&mut hidden, "felt", false),
            "{\"ok\":true,\"hidden\":[\"vista\"]}"
        );
    }

    /// The hide system gives back what it took, not `Inherited` by default.
    #[test]
    fn a_hidden_kind_comes_back_as_it_was() {
        #[derive(Component)]
        struct Marked;
        let mut app = App::new();
        app.init_resource::<Hidden>()
            .add_systems(Update, hide::<Marked>("marked"));
        let shown = app.world_mut().spawn((Marked, Visibility::Visible)).id();
        let other = app.world_mut().spawn(Visibility::Visible).id();
        set_hidden(
            &mut app.world_mut().resource_mut::<Hidden>(),
            "marked",
            true,
        );
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(shown),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world().get::<Visibility>(other),
            Some(&Visibility::Visible)
        );
        set_hidden(
            &mut app.world_mut().resource_mut::<Hidden>(),
            "marked",
            false,
        );
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(shown),
            Some(&Visibility::Visible)
        );
    }

    /// The counter counts: a fresh `Vec` with capacity is one allocation —
    /// but only where this allocator is installed, so the test asks for a
    /// difference of at least zero there and exactly nothing of it here.
    #[test]
    fn the_counters_never_run_backwards() {
        let (count_before, bytes_before, _) = allocations();
        let buffer: Vec<u8> = Vec::with_capacity(64);
        drop(buffer);
        let (count_after, bytes_after, _) = allocations();
        assert!(count_after >= count_before && bytes_after >= bytes_before);
    }
}
