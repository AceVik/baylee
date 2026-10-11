//! `cards/lands/utility/sokenzan_crucible_of_defiance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn sokenzan_crucible_of_defiance_taps_for_red_and_offers_channel_from_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                sokenzan_crucible_of_defiance(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[sokenzan_crucible_of_defiance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let on_field = on_battlefield(&engine, p0, sokenzan_crucible_of_defiance())
        .expect("sokenzan on battlefield");
    let in_hand_card =
        in_hand(&engine, p0, sokenzan_crucible_of_defiance()).expect("sokenzan in hand");

    // Float {{4}} red mana from the four Mountains while keeping Sokenzan untapped.
    tap_mana_except(&mut engine, p0, on_field);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4
    );
    assert!(!is_tapped(&engine, on_field));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&in_hand_card),
        "lands are not castable spells"
    );
    assert!(
        legal
            .abilities
            .iter()
            .any(|(source, _)| *source == in_hand_card),
        "channel is payable from hand"
    );
    assert!(
        legal.abilities.contains(&(on_field, 0)),
        "ability 0 ({{T}}: Add {{R}}) is offered"
    );

    activate(&mut engine, p0, sokenzan_crucible_of_defiance(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 5);
    assert_eq!(pool.total(), 5);
    assert!(is_tapped(&engine, on_field));
}
