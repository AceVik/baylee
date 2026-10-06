//! `cards/lands/utility/blast_zone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blast Zone: "This land enters with a charge counter on it." / "{{T}}: Add
/// {{C}}." / "{{X}}{{X}}, {{T}}: Put X charge counters on this land."
///
/// The counters are what the card is for — its third ability destroys every
/// nonland permanent whose mana value equals them, and that clause is the
/// whole of its remaining `Coverage::Partial`. The second ability was
/// therefore an ability that tapped the land, spent nothing and changed
/// nothing, which is the shape a test asserting only "it was offered" cannot
/// see.
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
/// Under `Coverage::Partial`, the `{3}, {T}` permanent destruction ability is omitted.
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
