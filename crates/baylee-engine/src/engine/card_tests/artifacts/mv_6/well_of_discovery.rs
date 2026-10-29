//! Well of Discovery — {6} — Artifact
//! Oracle: At the beginning of your end step, if you control no untapped lands, draw a card.

use super::*;

#[test]
fn well_of_discovery_draws_a_card_at_end_step_when_all_lands_are_tapped() {
    let well_of_discovery = card_index("d63185ac-c7cd-4501-9248-54d6caf7d091");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[well_of_discovery])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, well_of_discovery);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, well_of_discovery).is_some());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}

#[test]
fn well_of_discovery_does_not_trigger_when_an_untapped_land_remains() {
    let well_of_discovery = card_index("d63185ac-c7cd-4501-9248-54d6caf7d091");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[well_of_discovery])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let lands = all_on_battlefield(&engine, p0, island());
    for &land in &lands[..6] {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .expect("tap Island");
    }
    let card = in_hand(&engine, p0, well_of_discovery).expect("Well of Discovery in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("cast Well of Discovery");
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, lands[6]));
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
