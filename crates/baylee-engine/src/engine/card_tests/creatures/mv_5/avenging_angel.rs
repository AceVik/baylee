//! `cards/creatures/mv_5/avenging_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn avenging_angel() -> CardIndex {
    card_index("3415a199-05ac-4627-be97-f842eb1415a3")
}

/// Avenging Angel (3415a199-05ac-4627-be97-f842eb1415a3) — {3}{W}{W} Creature — Angel:
/// "Flying. When this creature dies, you may put it on top of its owner's library."
#[test]
fn avenging_angel_returns_to_top_of_library_when_it_dies() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[avenging_angel(), swamp(), swamp()])
        .hand(0, &[index::TERROR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P0, avenging_angel()).unwrap();
    cast_from_hand(&mut engine, P0, index::TERROR);
    aim_at(&mut engine, P0, angel);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, avenging_angel()).is_none());
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(avenging_angel())
    );
}
