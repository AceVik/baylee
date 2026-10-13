//! The tests of the cards under `cards/lands/planets/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod adagia_windswept_bastion;
mod evendo_waking_haven;
mod kavaron_memorial_world;
mod susur_secundi_void_altar;
mod uthros_titanic_godcore;

/// Walks a [`legends_kit`](crate::engine::card_tests::legends_kit) game from
/// player 0's main phase to the first moment of player 1's turn at which
/// player 0 holds priority: instant timing, never sorcery timing for p0
/// (CR 307.1, 117.1a), with the board exactly as it was.
#[track_caller]
fn to_instant_timing(e: &mut Engine<RegistryLookup>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(e, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        stack_is_empty(e),
        "an instant-timing window with an empty stack"
    );
}

/// Whether player 0 is offered ability `index` of `source` right now.
fn planet_offered(e: &Engine<RegistryLookup>, source: ObjectId, index: u32) -> bool {
    matches!(
        e.pending(),
        Pending::Priority { player, legal }
            if *player == PlayerId::new(0) && legal.abilities.contains(&(source, index))
    )
}

/// The direct attempt at ability `index` of `source`: refused, and the game
/// state is left exactly as it was.
#[track_caller]
fn assert_activation_refused(e: &mut Engine<RegistryLookup>, source: ObjectId, index: u32) {
    let before = e.state().snapshot_hash();
    assert!(
        e.apply(
            PlayerId::new(0),
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .is_err(),
        "ability {index} is refused outside sorcery timing"
    );
    assert_eq!(e.state().snapshot_hash(), before, "and nothing changed");
}
