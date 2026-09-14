//! The door Android comes in through.
//!
//! Android never calls `main`. It loads `libbaylee_client_android.so` and
//! calls `android_main`, handing it the `AndroidApp` that owns the window,
//! the input queue and the asset manager — so the entry point has to be an
//! exported symbol in a shared object, and a shared object is the one thing a
//! `[[bin]]` is not.
//!
//! `#[bevy_main]` writes that symbol: it stores the `AndroidApp` where bevy's
//! winit backend will look for it and then calls the function below it. Every
//! step after that is [`baylee_client::standalone::run`], the same three the
//! desktop binary takes — which is the point of this package being nine lines
//! and not a second copy of the client's startup.

#[cfg(target_os = "android")]
use bevy::prelude::bevy_main;

#[cfg(target_os = "android")]
#[bevy_main]
fn main() {
    baylee_client::standalone::run();
}
