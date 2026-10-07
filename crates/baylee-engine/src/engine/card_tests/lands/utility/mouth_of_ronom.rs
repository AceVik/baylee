//! `cards/lands/utility/mouth_of_ronom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mouth of Ronom: "{T}: Add {C}." / "{4}{S}, {T}, Sacrifice this land: It
/// deals 4 damage to target creature." Under `Coverage::Implemented`, paying
/// the whole cost and sacrificing the land deals 4 damage to a target
/// creature, and 4 damage destroys a 1/1 through state-based actions.
///
/// A Snow-Covered Forest provides green mana for `{S}`, while four ordinary
/// Forests pay `{4}`. Colorless mana is neither required nor sufficient.
/// `mana_pay::tests::issue_158_snow_is_provenance_not_color` also covers
/// colored snow mana and rejects ordinary colorless mana.
#[test]
fn mouth_of_ronom_sacrifices_to_deal_damage_to_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let snow_forest =
        baylee_cards::decks::by_name("Snow-Covered Forest").expect("snow land in pool");
    let mut engine = Duel::new(132, forest())
        .battlefield(
            0,
            &[
                mouth_of_ronom(),
                snow_forest,
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mouth = on_battlefield(&engine, p0, mouth_of_ronom()).expect("Mouth");
    let spare = on_battlefield(&engine, p0, snow_forest).expect("snow source");
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("elf deployed");

    // Colored snow mana must pay {S}; ordinary green mana pays only {4}.
    // This fails when snow is incorrectly treated as colorless (#158).
    let taken = tap_mana_except(&mut engine, p0, mouth);
    assert_eq!(taken, 5, "four Forests and a Snow-Covered Forest");
    assert!(
        is_tapped(&engine, spare),
        "the snow land provided green snow mana"
    );

    activate(&mut engine, p0, mouth_of_ronom(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert!(in_graveyard(&engine, p0, mouth_of_ronom()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, quiet_creature()).is_some());
    assert!(on_battlefield(&engine, p1, quiet_creature()).is_none());
}
