//! `cards/creatures/mv_4/sailmonger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "b15af529-a61a-4228-bf2a-7304c9050720"
fn sailmonger() -> CardIndex {
    card_index("b15af529-a61a-4228-bf2a-7304c9050720")
}

/// `Sailmonger` (`Coverage::Generated`):
/// "{2}: Target creature gains flying until end of turn. Any player may activate this ability."
///
/// Verifies that an opponent can activate `Sailmonger`'s `{2}` ability to give
/// a creature flying until end of turn.
#[test]
fn sailmonger_grants_flying_when_activated_by_any_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9613, island())
        .battlefield(0, &[sailmonger()])
        .battlefield(1, &[index::GRIZZLY_BEARS, island(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    let monger = on_battlefield(&engine, p0, sailmonger()).expect("P0 Sailmonger");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    assert!(!keywords(&engine, bears).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: monger,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, bears).contains(KeywordSet::FLYING),
        "Sailmonger grants flying until end of turn when activated by any player"
    );
}
