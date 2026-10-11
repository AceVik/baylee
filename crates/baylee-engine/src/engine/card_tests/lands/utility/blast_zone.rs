//! `cards/lands/utility/blast_zone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blast Zone: "This land enters with a charge counter on it." / "{{T}}: Add
/// {{C}}." / "{{X}}{{X}}, {{T}}: Put X charge counters on this land."
///
/// The announced X places exactly that many counters, before replacement effects.
#[test]
fn blast_zone_adds_the_charge_counters_it_announces() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blast_zone(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let zone = on_battlefield(&engine, p0, blast_zone()).expect("zone on battlefield");
    assert_eq!(
        counters_on(&engine, zone, CounterKind::Charge),
        1,
        "it enters with one"
    );

    tap_all_mana_but(&mut engine, p0, Some(blast_zone()));
    activate(&mut engine, p0, blast_zone(), 1);
    let Pending::ChooseNumber { max, .. } = engine.pending().clone() else {
        panic!("expected the announced X, got {:?}", engine.pending());
    };
    assert_eq!(max, 2);
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("announce X = 2");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, zone, CounterKind::Charge),
        3,
        "the one it entered with, plus the two announced"
    );
    assert!(is_tapped(&engine, zone));
}

/// Blast Zone: "This land enters with a charge counter on it." / "{T}: Add {C}." / "{X}{X}, {T}: Put X charge counters on this land..."
/// Playing this land causes it to enter with a charge counter and immediately tap for colorless mana.
#[test]
fn blast_zone_enters_with_charge_counter_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(331, forest()).hand(0, &[blast_zone()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, blast_zone());
    assert_eq!(counters_on(&engine, land, CounterKind::Charge), 1);

    activate(&mut engine, p0, blast_zone(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn blast_zone_sweeps_its_last_known_counter_value_after_paying_the_sacrifice() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for charge in [0, 1, 2] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    blast_zone(),
                    forest(),
                    forest(),
                    forest(),
                    index::MOX_EMERALD,
                ],
            )
            .battlefield(1, &[llanowar_elves(), index::GRIZZLY_BEARS, forest()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let zone = on_battlefield(&e, p0, blast_zone()).unwrap();
        let zero = on_battlefield(&e, p0, index::MOX_EMERALD).unwrap();
        let one = on_battlefield(&e, p1, llanowar_elves()).unwrap();
        let two = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
        let land = on_battlefield(&e, p1, forest()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        crate::replacement::remove_counters(state, zone, CounterKind::Charge, 1);
        crate::replacement::put_counters(state, zone, CounterKind::Charge, charge);
        tap_all_mana_but(&mut e, p0, Some(blast_zone()));
        activate(&mut e, p0, blast_zone(), 2);
        assert_eq!(e.state().object(zone).unwrap().zone, Zone::Graveyard);
        pass_until(&mut e, stack_is_empty);
        for (object, value) in [(zero, 0), (one, 1), (two, 2)] {
            assert_eq!(
                e.state().object(object).unwrap().zone == Zone::Graveyard,
                value == charge,
                "charge {charge}, mana value {value}"
            );
        }
        assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    }
}
