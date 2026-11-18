//! The audio thread never allocates while the recorded soundtrack streams.
//!
//! A counting allocator counts the allocations made on the thread that is
//! being watched (the test's own, standing in for the audio thread); the
//! worker that decodes, and allocates inside the Opus decoder, is another
//! thread and not counted. This binary holds this one test, so nothing else
//! runs on a watched thread.
#![cfg(not(target_arch = "wasm32"))]
use baylee_client_core::music::{RATE, Scene, ScoreControl, ScoreRequest, Tune};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static WATCHED: Cell<bool> = const { Cell::new(false) };
}

// SAFETY: every call goes straight to `System` with the arguments it was
// given, so the allocator's contract is `System`'s. The counter and the
// const-initialised thread-local flag allocate nothing.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if WATCHED.with(Cell::get) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the caller's layout is passed on unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System.alloc` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if WATCHED.with(Cell::get) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: as `alloc` and `dealloc`: the caller's arguments, unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Two seconds of the streaming tune, pulled in device-sized blocks at
/// about the device's pace, through a scene change and a stinger: the
/// pulling thread allocates nothing, and the recording is heard.
#[test]
fn the_audio_thread_never_allocates_while_the_recording_streams() {
    let lobby = ScoreRequest {
        scene: Scene::Lobby,
        recorded: true,
        ..ScoreRequest::default()
    };
    let control = Arc::new(ScoreControl::default());
    control.set(lobby);
    let mut tune = Tune::streaming(control.clone());
    let mut block = [[0.0f32; 2]; 512];
    let pace = Duration::from_micros(512 * 1_000_000 / u64::from(RATE));
    // Warm-up: the worker indexes the files and fills its ring.
    std::thread::sleep(Duration::from_millis(500));
    for _ in 0..20 {
        tune.render(&mut block);
    }
    let mut loudest = 0.0f32;
    WATCHED.with(|watched| watched.set(true));
    for k in 0..(2 * RATE as usize / block.len()) {
        if k == 40 {
            control.set(ScoreRequest {
                scene: Scene::Table,
                ..lobby
            });
        }
        if k == 120 {
            control.set(ScoreRequest {
                scene: Scene::Table,
                stingers: 1,
                stinger_epic: true,
                ..lobby
            });
        }
        tune.render(&mut block);
        loudest = block
            .iter()
            .map(|frame| frame[0].abs().max(frame[1].abs()))
            .fold(loudest, f32::max);
        std::thread::sleep(pace);
    }
    WATCHED.with(|watched| watched.set(false));
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
    assert!(loudest > 0.01, "the recording was heard: {loudest}");
}
