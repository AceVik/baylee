//! `cards/lands/utility/myriad_landscape.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Myriad Landscape prints `This land enters tapped.`, `{{T}}: Add {{C}}.`, and `{{2}}, {{T}}, Sacrifice this land: Search your library for up to two basic land cards that share a land type, put them onto the battlefield tapped, then shuffle.`
///
/// Under `Coverage::Partial`, the basic land tutor ability is omitted because no filter can express the relational constraint that two searched cards must share a land type.
/// Playing Myriad Landscape from hand enters tapped.
/// In the next turn, with `{{2}}` floating from two `forest()` lands and Myriad Landscape untapped, ability 0 is offered while ability 1 is withheld from `legal.abilities`.
/// Activating ability 0 produces one colorless mana and taps the land.
#[test]
fn myriad_landscape_enters_tapped_and_omits_basic_land_tutor() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[myriad_landscape()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let landscape = play_land(&mut engine, p0, myriad_landscape());
    assert!(
        entered_tapped(&engine, landscape),
        "myriad landscape enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, landscape));

    // Float {{2}} from Forests while keeping Myriad Landscape untapped.
    tap_mana_except(&mut engine, p0, landscape);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(landscape, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(landscape, 1)),
        "ability 1 is omitted under `Coverage::Partial` even with {{2}} floating"
    );

    activate(&mut engine, p0, myriad_landscape(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Green), 2);
    assert_eq!(pool.total(), 3);
    assert!(is_tapped(&engine, landscape));
}
