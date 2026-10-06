//! `cards/lands/utility/sokenzan_crucible_of_defiance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sokenzan, Crucible of Defiance` prints `{{T}}: Add {{R}}.` and `Channel — {{3}}{{R}}, Discard this card: Create two 1/1 colorless Spirit creature tokens. They gain haste until end of turn. This ability costs {{1}} less to activate for each legendary creature you control.`
///
/// Under `Coverage::Partial`, only the red mana ability is implemented while the channel ability is omitted.
/// With a copy on the battlefield and another in hand, floating `{{3}}{{R}}` from four `mountain()` lands confirms that no channel activation is offered from hand in `legal.abilities` and the card is not castable.
/// The battlefield copy taps for one red mana and becomes tapped.
#[test]
fn sokenzan_crucible_of_defiance_taps_for_red_and_omits_channel_from_hand() {
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
        !legal
            .abilities
            .iter()
            .any(|(source, _)| *source == in_hand_card),
        "channel ability is omitted under `Coverage::Partial` despite floating {{3}}{{R}}"
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
