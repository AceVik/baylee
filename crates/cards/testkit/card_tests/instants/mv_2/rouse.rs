//! `cards/instants/mv_2/rouse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "475099d9-4743-420a-ada8-3932f5ef9e82"
fn rouse() -> CardIndex {
    card_index("475099d9-4743-420a-ada8-3932f5ef9e82")
}

/// `Rouse` (`Coverage::Partial`):
/// "Target creature gets +2/+0 until end of turn."
///
/// Verifies that casting `Rouse` for `{1}{B}` gives target creature +2/+0
/// until end of turn.
#[test]
fn rouse_gives_target_creature_plus_two_plus_zero_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9602, swamp())
        .battlefield(0, &[index::GRIZZLY_BEARS, swamp(), swamp()])
        .hand(0, &[rouse()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("P0 Grizzly Bears");
    assert_eq!(pt(&engine, bears), (2, 2));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, rouse());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (4, 2),
        "Rouse gives target creature +2/+0 until end of turn"
    );
}
