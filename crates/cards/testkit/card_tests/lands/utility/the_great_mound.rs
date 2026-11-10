//! `cards/lands/utility/the_great_mound.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn the_great_mound_draws_card_for_six_and_offers_vibranium_token_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                the_great_mound(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mound = on_battlefield(&engine, p0, the_great_mound()).expect("mound on battlefield");

    // Float {{6}} from Forests while keeping The Great Mound untapped.
    tap_mana_except(&mut engine, p0, mound);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);
    assert!(!is_tapped(&engine, mound));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(mound, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(mound, 1)),
        "ability 1 ({{6}}, {{T}}: Draw a card) is offered"
    );
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(source, _)| *source == mound)
            .count(),
        3,
        "both mana and nonmana activations are offered"
    );

    let lib_before = library_size(&engine, p0);

    activate(&mut engine, p0, the_great_mound(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), lib_before - 1, "drew a card");
    assert!(is_tapped(&engine, mound));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
