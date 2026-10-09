//! Standalone duel client.
//!
//! Everything it does is [`baylee_client::standalone::run`]. The body sits in
//! the library because a phone never calls `main`: Android loads a shared
//! object and calls `android_main`, so the same three steps have to be
//! reachable from somewhere that is not a binary.
//!
//! A Windows release build opens no console beside its window; `--console`
//! asks for one (`baylee_client::console`).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

/// A `dev-control` build counts its allocations for `/perf`
/// (`devctl::perf`); every other build keeps the system allocator untouched.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
#[global_allocator]
static ALLOCATOR: baylee_client::devctl::CountingAlloc = baylee_client::devctl::CountingAlloc;

fn main() {
    baylee_client::console::open_if_asked();
    baylee_client::standalone::run();
}
