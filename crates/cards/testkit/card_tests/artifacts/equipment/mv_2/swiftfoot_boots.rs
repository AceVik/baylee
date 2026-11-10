//! `cards/artifacts/equipment/mv_2/swiftfoot_boots.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// CR 704.5n against CR 704.5m: an Equipment whose host stops being a
/// creature comes **unattached and stays on the battlefield**, where an Aura
/// in the same position goes to its owner's graveyard.
///
/// The asymmetry is the whole test. Both are attachments, both have a host
/// that is still a permanent and still on the battlefield, and the only
/// thing that differs is which rule the permanent's own subtype puts it
/// under — CR 301.5b says an Equipment attaches to a creature, so a host
/// that is no longer one is illegal for it (CR 301.5c). A fix that read
/// "the host is illegal" and reached for one outcome would put the Boots in
/// the graveyard, and nothing about the Boots themselves would look wrong.
#[test]
fn swiftfoot_boots_come_off_a_host_that_stops_being_a_creature_and_stay_on_the_table() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                swiftfoot_boots(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[swift_reconfiguration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves deployed");
    let boots = on_battlefield(&engine, p0, swiftfoot_boots()).expect("the Boots are out");

    // Equip {1} (CR 702.6a). The Elves keep their own mana ability untapped;
    // the three Plains are the pool, and one of them pays for the boots.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    // Ability 0 is the static that grants hexproof and haste, 1 the equip.
    activate(&mut engine, p0, swiftfoot_boots(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(boots).and_then(|o| o.attached_to),
        Some(host),
        "the Boots are on the Elves"
    );

    // Hexproof is the opponent's word (CR 702.11b), so my own Aura may still
    // aim at the creature wearing them.
    cast_from_hand(&mut engine, p0, swift_reconfiguration());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let now = types(&engine, host);
    assert!(
        !now.contains(TypeSet::CREATURE),
        "the host is a Vehicle and no creature: {now:?}"
    );
    assert!(
        on_battlefield(&engine, p0, swiftfoot_boots()).is_some(),
        "an Equipment with an illegal host stays on the battlefield \
         (CR 704.5n)"
    );
    assert_eq!(
        engine.state().object(boots).and_then(|o| o.attached_to),
        None,
        "and comes unattached from it"
    );
}
