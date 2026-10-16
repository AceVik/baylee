//! `cards/creatures/artifacts/mv_3/darksteel_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn darksteel_myr() -> CardIndex {
    card_index("f90fe86c-5cca-483b-93c2-6e856fa01c88")
}

/// Bolt `target` from seat 0's hand and let it resolve.
fn bolt(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    let p0 = PlayerId::new(0);
    cast_from_hand(engine, p0, lightning_bolt());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(engine, p0, target);
    pass_until(engine, stack_is_empty);
}

/// Darksteel Myr — 0/1 indestructible. Lethal damage does not destroy it
/// (the state-based action destroys, and indestructible says it cannot), where
/// the same bolt kills an ordinary 1/1 on the same board.
#[test]
fn darksteel_myr_survives_lethal_damage_where_a_plain_creature_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .battlefield(
            0,
            &[mountain(), mountain(), darksteel_myr(), quiet_creature()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let myr = on_battlefield(&engine, p0, darksteel_myr()).expect("seated");
    let plain = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    assert_eq!(pt(&engine, myr), (0, 1));
    assert!(keywords(&engine, myr).contains(KeywordSet::INDESTRUCTIBLE));

    bolt(&mut engine, plain);
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "three damage kills the ordinary 1/1"
    );

    bolt(&mut engine, myr);
    assert!(
        on_battlefield(&engine, p0, darksteel_myr()).is_some(),
        "three damage on a 0/1 does not destroy an indestructible creature"
    );
    assert!(in_graveyard(&engine, p0, darksteel_myr()).is_none());
}

/// "Effects that say destroy don't destroy this creature": Vindicate aimed at
/// the Myr resolves and the Myr stays.
#[test]
fn darksteel_myr_is_not_destroyed_by_a_destroy_effect() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[vindicate()])
        .battlefield(0, &[plains(), swamp(), forest(), darksteel_myr()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let myr = on_battlefield(&engine, p0, darksteel_myr()).expect("seated");

    cast_from_hand(&mut engine, p0, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, myr);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, darksteel_myr()).is_some());
    assert!(in_graveyard(&engine, p0, darksteel_myr()).is_none());
}
