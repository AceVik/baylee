//! `cards/creatures/artifacts/mv_2/prismite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn prismite() -> CardIndex {
    card_index("feb4fcec-6f3a-4643-9ffb-69d3f5953322")
}

/// Prismite: a 2/1 with "{2}: Add one mana of any color". It needs two
/// mana, spends them, and leaves one of the named color; there is no tap in
/// the price, so it can be used again with more mana.
#[test]
fn prismite_turns_two_mana_into_one_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[prismite(), forest(), forest(), forest(), forest()],
        &[],
    );
    let me = on_battlefield(&engine, p0, prismite()).expect("seated");
    assert_eq!(pt(&engine, me), (2, 1));
    let forests = all_on_battlefield(&engine, p0, forest());
    let offered = |e: &Engine<RegistryLookup>| priority_offer(e).abilities.contains(&(me, 0));
    assert!(!offered(&engine), "nothing floating");

    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    assert!(!offered(&engine), "one mana does not pay {{2}}");
    tap_mana_where(&mut engine, p0, |id| id == forests[1]);
    assert!(offered(&engine), "two do");
    activate(&mut engine, p0, prismite(), 0);
    name_a_color(&mut engine, ManaColor::Black);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the two green were paid"
    );
    assert_eq!(pool.total(), 1);
    assert!(!is_tapped(&engine, me), "no tap in the price");

    tap_mana_where(&mut engine, p0, |id| id == forests[2] || id == forests[3]);
    assert!(offered(&engine), "again, with two more");
    activate(&mut engine, p0, prismite(), 0);
    name_a_color(&mut engine, ManaColor::White);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the second named color"
    );
    assert_eq!(
        pool.total(),
        2,
        "three floating, two paid, the new one added"
    );
}
