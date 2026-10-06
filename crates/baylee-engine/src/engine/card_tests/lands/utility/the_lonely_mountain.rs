//! `cards/lands/utility/the_lonely_mountain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `The Lonely Mountain` prints `({{T}}: Add {{R}}.)`, `This land enters tapped unless you control an Equipment.`, and `{{4}}{{R}}, {{T}}: Create a 2/2 red Dwarf creature token. This ability costs {{1}} less to activate for each Equipment you control. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, `EnterModifier::TappedUnless` and the `{{T}}: Add {{R}}` mana ability are built, while the Dwarf token creation and cost reduction are omitted.
/// Without an Equipment on the battlefield, playing this land puts it onto the battlefield tapped.
/// After untapping on the next turn, floating `{{5}}` red mana from basic mountains verifies that ability 0 is offered while ability 1 is withheld under `Coverage::Partial`, and tapping `The Lonely Mountain` adds `{{R}}`.
#[test]
fn the_lonely_mountain_enters_tapped_without_equipment_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[the_lonely_mountain()])
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_lonely_mountain());
    assert!(
        entered_tapped(&engine, land),
        "without an Equipment, The Lonely Mountain enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    // Float {{5}} red mana from basic mountains while keeping The Lonely Mountain untapped.
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        5
    );
    assert!(!is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "ability 0 ({{T}}: Add {{R}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "Dwarf token ability is omitted under `Coverage::Partial` despite floating {{4}}{{R}}"
    );

    activate(&mut engine, p0, the_lonely_mountain(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 6);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, land));
}
