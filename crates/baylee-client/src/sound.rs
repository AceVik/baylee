//! The sink: where a decided cue would become a noise.
//!
//! [`baylee_client_core::cue`] is the whole of the thinking — which moments
//! are worth hearing, and the arithmetic that makes a triple block one sound
//! instead of three. This is the other half, and today it is **deliberately
//! silent**: it drains the queue, remembers the last cue so `/state` can be
//! read instead of listened to, and hands each one to [`sound`], which does
//! nothing.
//!
//! # Why it stops here
//!
//! Two questions are open and both of them are the owner's, and they are the
//! same question wearing two hats:
//!
//! 1. **Where the sounds come from.** `docs/legal.md` has four clauses —
//!    the code licence, the `WotC` fan policy, Scryfall, privacy — and none of
//!    them is about audio. Shipped CC0 files and generated tones are
//!    different answers with different consequences, and the felt and the
//!    parchment were *arithmetic* for exactly the reason §2 gives: ornament
//!    is the easiest thing to borrow by accident.
//! 2. **What playing anything costs the build.** The workspace's bevy
//!    feature list has no `bevy_audio`, so the client links no audio backend
//!    at all. Adding it pulls rodio and cpal — `CoreAudio`, ALSA, WASAPI —
//!    into a binary that CI runs through `cargo-deny`, `cargo-audit`, an
//!    MSRV check against 1.88 and a `wasm32` check, and on the web an
//!    `AudioContext` does not start until the player has clicked something.
//!
//! Neither is a thing to decide by writing a line of code, so the seam is the
//! deliverable. Everything above it is already true and already tested: the
//! cues are decided, deduplicated, retracted when the client answers its own
//! question, and drained on a schedule. When the two answers arrive, [`sound`]
//! is the one function that changes.

use bevy::prelude::*;

use crate::Duel;
use baylee_client_core::cue::Cue;

/// Hands this frame's cues to the sink and remembers the last of them.
///
/// In `DuelSet::Present` and not in `Sync`, which is what makes
/// [`baylee_client_core::cue::Cues::retract`] work at all: every source of a
/// cue and everything that answers a question have both run by the time this
/// does, so a chime the client withdrew inside the frame never reaches the
/// sink.
///
/// The early return is not a micro-optimisation. `Duel` is a resource and
/// touching it mutably marks it changed, so a drain that ran unconditionally
/// would report the whole duel as having moved on every frame of a game where
/// nothing happened at all.
pub fn play_the_cues(mut duel: ResMut<Duel>) {
    if duel.cues.pending().is_empty() {
        return;
    }
    for cue in duel.cues.take() {
        sound(cue);
    }
}

/// Makes the noise a cue asks for — which is, for now, none.
///
/// A function rather than an empty loop body so that the silence has a name,
/// a doc comment and one call site. See the module header for the two
/// questions it is waiting on.
fn sound(cue: Cue) {
    // The signature the sink will have once it has a device to play on; a
    // `Cue` is one `Copy` discriminant, so taking it by value costs nothing
    // and reads as what it is.
    let _ = cue;
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::interaction::Outcome;

    /// What [`watch`] saw on the frame that just ran.
    ///
    /// "Did this frame mark the duel dirty" can only be asked from *inside*
    /// the frame. `App::update` ends with `World::clear_trackers`, which moves
    /// `last_change_tick` past every write the update made, so a `Ref<Duel>`
    /// taken afterwards answers `false` whatever the systems did — the first
    /// draft of the counter-test below asked from outside and passed with the
    /// early return deleted.
    #[derive(Resource, Default)]
    struct Dirtied(bool);

    /// Runs after [`play_the_cues`] and records whether it moved the tick.
    fn watch(duel: Res<Duel>, mut dirtied: ResMut<Dirtied>) {
        dirtied.0 = duel.is_changed();
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Duel>()
            .init_resource::<Dirtied>()
            .add_systems(Update, (play_the_cues, watch).chain());
        app
    }

    /// The drain is wired, which is the only thing about a silent sink a test
    /// can say — and the thing that is worth saying, because "decided but
    /// never drained" is a queue that grows for the length of a game.
    #[test]
    fn a_decided_cue_is_drained_by_the_frame_after_it() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<Duel>()
            .cues
            .note_ending(Outcome::YouLost);
        assert_eq!(
            app.world().resource::<Duel>().cues.pending(),
            [Cue::GameLost]
        );
        app.update();
        let duel = app.world().resource::<Duel>();
        assert!(duel.cues.pending().is_empty(), "the queue was drained");
        assert_eq!(duel.cues.last(), Some(Cue::GameLost), "and remembered");
    }

    /// The counter-test for the early return: a frame with nothing to say
    /// must not report the duel as having changed, or every reader that
    /// watches the resource rebuilds on every frame of a quiet game.
    ///
    /// The second half is the counter-test's own counter-test. A watcher that
    /// can never see dirt would pass the first assertion however wrong the
    /// system was, so the cue goes in through `bypass_change_detection` —
    /// leaving [`play_the_cues`] as the only thing that can have moved the
    /// tick on the frame after it.
    #[test]
    fn a_silent_frame_does_not_touch_the_duel() {
        let mut app = app();
        // `init_resource` marked it; that is not a frame's doing.
        app.update();
        app.update();
        assert!(
            !app.world().resource::<Dirtied>().0,
            "a frame with no cues in it marked the whole duel dirty"
        );

        app.world_mut()
            .resource_mut::<Duel>()
            .bypass_change_detection()
            .cues
            .note_refusal();
        app.update();
        assert!(
            app.world().resource::<Dirtied>().0,
            "the watcher never sees dirt, so the assertion above proves nothing"
        );
    }
}
