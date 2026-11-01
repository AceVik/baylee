//! `cards/instants/mv_3/invigorate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "9264c198-76c7-4ac0-9334-37c691de1b54"
fn invigorate() -> CardIndex {
    card_index("9264c198-76c7-4ac0-9334-37c691de1b54")
}

/// `Invigorate` (`Coverage::Partial` — mana-cost mode `{2}{G}`):
/// "Target creature gets +4/+4 until end of turn."
///
/// Verifies that `Invigorate` gives the targeted creature +4/+4 until end of
/// turn without affecting other creatures, and wears off at cleanup.
#[test]
fn invigorate_pumps_target_creature_plus_four_plus_four_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9504, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[invigorate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("P0 Grizzly Bears");
    let bystander = on_battlefield(&engine, p0, quiet_creature()).expect("P0 quiet creature");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, invigorate());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&bear));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bear),
        (6, 6),
        "2/2 gets +4/+4 until end of turn"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "untouched creature stays 1/1"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bear), (2, 2), "+4/+4 wears off at end of turn");
}
