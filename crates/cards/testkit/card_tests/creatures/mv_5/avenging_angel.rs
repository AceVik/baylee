//! `cards/creatures/mv_5/avenging_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn avenging_angel() -> CardIndex {
    card_index("3415a199-05ac-4627-be97-f842eb1415a3")
}

fn terror() -> CardIndex {
    card_index("b81f041d-98db-4408-9472-c483e4a502bc")
}

/// p0 has an Avenging Angel, two Swamps and a Terror in hand. Terror is cast
/// at the Angel and the game stops on the Angel's "you may" question.
fn the_angel_dies_to_terror() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[avenging_angel(), swamp(), swamp()])
        .hand(0, &[terror()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P0, avenging_angel()).unwrap();
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));
    cast_from_hand(&mut engine, P0, terror());
    aim_at(&mut engine, P0, angel);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the Angel's controller decides");
    engine
}

/// The card on top of `seat`'s library.
fn top_of_library(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Option<CardIndex> {
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .last()?;
    engine.state().object(top).unwrap().card.map(|c| c.index)
}

/// Avenging Angel (3415a199-05ac-4627-be97-f842eb1415a3) — {3}{W}{W} Creature — Angel:
/// "Flying. When this creature dies, you may put it on top of its owner's library."
#[test]
fn avenging_angel_returns_to_top_of_library_when_it_dies_and_its_controller_says_yes() {
    let mut engine = the_angel_dies_to_terror();

    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, avenging_angel()).is_none());
    assert_eq!(top_of_library(&engine, P0), Some(avenging_angel()));
}

#[test]
fn avenging_angel_stays_in_the_graveyard_when_its_controller_says_no() {
    let mut engine = the_angel_dies_to_terror();

    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, avenging_angel()).is_some());
    assert_ne!(
        top_of_library(&engine, P0),
        Some(avenging_angel()),
        "a \"may\" declined leaves the library alone"
    );
}
