//! `cards/lands/utility/magosi_the_waterveil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Magosi, the Waterveil prints `This land enters tapped.`, `{{T}}: Add {{U}}.`, `{{U}}, {{T}}: Put an eon counter on this land. Skip your next turn.`, and `{{T}}, Remove an eon counter from this land and return it to its owner's hand: Take an extra turn after this one.`
///
/// Under `Coverage::Partial`, the eon-counter abilities are omitted because no effect skips a turn and eon counters are not defined in `counters`.
/// Playing Magosi, the Waterveil from hand enters tapped.
/// In the next turn, with `{{U}}` floating from an `island()` and Magosi untapped, ability 0 is offered while abilities 1 and 2 are omitted from `legal.abilities`.
/// Activating ability 0 adds `{{U}}` to `pool.available(ManaColor::Blue)` and taps the land.
#[test]
fn magosi_the_waterveil_enters_tapped_and_omits_eon_counter_abilities() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[magosi_the_waterveil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let magosi = play_land(&mut engine, p0, magosi_the_waterveil());
    assert!(
        entered_tapped(&engine, magosi),
        "magosi, the waterveil enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, magosi));

    // Float {{U}} from Island while keeping Magosi untapped.
    tap_mana_except(&mut engine, p0, magosi);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(magosi, 0)),
        "ability 0 ({{T}}: Add {{U}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(magosi, 1)),
        "ability 1 is omitted under `Coverage::Partial` even with {{U}} floating"
    );
    assert!(
        !legal.abilities.contains(&(magosi, 2)),
        "ability 2 is omitted under `Coverage::Partial`"
    );

    activate(&mut engine, p0, magosi_the_waterveil(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 2);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, magosi));
}
