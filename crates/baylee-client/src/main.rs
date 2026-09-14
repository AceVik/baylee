//! Standalone duel client.
//!
//! Everything it does is [`baylee_client::standalone::run`]. The body sits in
//! the library because a phone never calls `main`: Android loads a shared
//! object and calls `android_main`, so the same three steps have to be
//! reachable from somewhere that is not a binary.

fn main() {
    baylee_client::standalone::run();
}
