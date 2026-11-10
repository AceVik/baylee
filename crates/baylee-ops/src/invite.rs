//! `baylee-invite`, the closed beta's key command (#317), after the beta
//! went open (owner, 10.11.2026): its keys are gone, and so is everything
//! it did. It says so and runs nothing.
//!
//! It is still built and installed as `/usr/local/sbin/baylee-invite` for
//! one reason: the `baylee-deploy` already on a server stages and installs
//! it by name, and that deploy is the one that brings this release. Without
//! the file the stage would install whatever an earlier build left in
//! `target/release`, the wrapper that ran `baylee-gateway invite` with the
//! gateway's own settings. Once every server has deployed this release, the
//! command and its place in the deploy can go.

/// What `baylee-invite` says, whatever it was asked.
pub const GONE: &str = "baylee-invite: the closed beta's keys are gone (the beta is open); \
     there is nothing to make, list or revoke. BAYLEE_REGISTRATION=open in \
     /etc/baylee/gateway.env lets new players in, off lets nobody new in.";

/// Says [`GONE`] on standard error and answers the exit status: 2, a
/// command used wrongly, whatever `_args` were.
#[must_use]
pub fn main(_args: &[String]) -> i32 {
    eprintln!("{GONE}");
    2
}
