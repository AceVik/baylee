//! `cards/creatures/mv_1/orcish_spy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn choose_targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().expect("a question is pending");
    if matches!(e.pending(), Pending::ChoosePlayer { .. }) {
        e.apply(p, PlayerAction::ChoosePlayer(players[0]))
            .expect("player chosen");
    } else {
        e.apply(p, PlayerAction::ChooseTargets { objects, players })
            .expect("targets chosen");
    }
}

fn ready() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("0d058991-3751-4afe-80c9-79f0c6e10282")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let spy = on_battlefield(&e, P0, card_index("0d058991-3751-4afe-80c9-79f0c6e10282"))
        .expect("the Spy is out");
    (e, spy)
}

/// Taps the Spy at `target` and returns the three cards it is shown.
fn look_at(e: &mut Engine<RegistryLookup>, spy: ObjectId, target: PlayerId) -> Vec<ObjectId> {
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: spy,
            ability_index: 0,
        },
    )
    .expect("{T} is paid");
    choose_targets(e, vec![], vec![target]);
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                player: P0,
                prompt: ChoicePrompt::LookAtLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on the look")
    };
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("nothing is taken");
    pass_until(e, stack_is_empty);
    options
}

/// The three cards at either end of `seat`'s library list.
fn ends_of_library(e: &Engine<RegistryLookup>, seat: PlayerId) -> [Vec<ObjectId>; 2] {
    let lib = e.state().zones.list(ZoneLocation::Library(seat)).clone();
    [
        lib.iter().copied().take(3).collect(),
        lib.iter().rev().copied().take(3).collect(),
    ]
}

/// Whether `seen` is exactly the same three cards as one end of the list.
fn is_one_end(seen: &[ObjectId], ends: &[Vec<ObjectId>; 2]) -> bool {
    ends.iter()
        .any(|end| seen.len() == end.len() && seen.iter().all(|id| end.contains(id)))
}

/// Orcish Spy (0d058991-3751-4afe-80c9-79f0c6e10282) — "{T}: Look at the top
/// three cards of target player's library."
#[test]
fn orcish_spy_shows_the_top_three_of_an_opponents_library_and_changes_nothing() {
    let (mut e, spy) = ready();
    let ends = ends_of_library(&e, P1);
    let size = library_size(&e, P1);
    let seq_before = e.state().journal.last_seq();

    let seen = look_at(&mut e, spy, P1);

    assert_eq!(seen.len(), 3);
    assert!(
        is_one_end(&seen, &ends),
        "the three shown are the top three: {seen:?} vs {ends:?}"
    );
    assert_eq!(library_size(&e, P1), size, "looking moves nothing");
    assert!(
        e.state()
            .object(spy)
            .expect("the Spy is out")
            .status
            .contains(Status::TAPPED)
    );
    let any_revealed = e.state().journal.entries().iter().any(|entry| {
        entry.seq > seq_before && matches!(entry.event, crate::event::GameEvent::Revealed { .. })
    });
    assert!(!any_revealed, "a look is private, nothing is revealed");
}

#[test]
fn orcish_spy_may_look_at_its_own_librarys_top_three() {
    let (mut e, spy) = ready();
    let ends = ends_of_library(&e, P0);
    let seen = look_at(&mut e, spy, P0);
    assert_eq!(seen.len(), 3);
    assert!(is_one_end(&seen, &ends), "{seen:?} vs {ends:?}");
}
