//! `cards/creatures/artifacts/mv_3/esper_battlemage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn esper_battlemage() -> CardIndex {
    card_index("2551f383-7308-4670-9fda-406b20e372a5")
}

/// Esper Battlemage — "{W}, {T}: Prevent the next 2 damage that would be dealt
/// to you this turn." Activated, a Lightning Bolt at its own controller deals
/// 1 instead of 3.
#[test]
fn esper_battlemage_prevents_the_next_two_damage_to_you() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[esper_battlemage(), plains(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mage = on_battlefield(&engine, p0, esper_battlemage()).expect("seated");
    assert_eq!(pt(&engine, mage), (2, 2));
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, esper_battlemage(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, mage));

    cast_with_floating(&mut engine, p0, lightning_bolt());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at_player(&mut engine, p0, p0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, 0), 19, "3 damage less the 2 prevented");
}

/// "{B}, {T}: Target creature gets -1/-1 until end of turn." Aimed at a 1/1 it
/// dies; the source taps.
#[test]
fn esper_battlemage_shrinks_a_creature_by_one_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[esper_battlemage(), swamp()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mage = on_battlefield(&engine, p0, esper_battlemage()).expect("seated");
    let victim = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    assert_eq!(pt(&engine, victim), (1, 1));
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, esper_battlemage(), 1);
    aim_at(&mut engine, p0, victim);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "-1/-1 killed the 1/1"
    );
    assert!(is_tapped(&engine, mage));
}
