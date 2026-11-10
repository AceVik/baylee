//! `cards/artifacts/mv_3/astrolabe.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn astrolabe() -> CardIndex {
    card_index("d19567ac-1f52-40eb-b2cf-4c791823ec92")
}

/// `Astrolabe` ({3}, Artifact) activates for `{1}, {T}, Sacrifice this artifact`
/// to add two mana of any one color and draw a card at the beginning of the
/// next turn's upkeep.
#[test]
fn astrolabe_sacrifices_for_two_mana_of_one_color_and_draws_next_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 4])
        .hand(0, &[astrolabe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, astrolabe());
    pass_until(&mut engine, stack_is_empty);
    let astro = on_battlefield(&engine, p0, astrolabe()).expect("Astrolabe enters");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: astro,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, astrolabe()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );

    // Only the next turn's upkeep: P0's own draw step on the turn after is the ordinary one card.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "the Astrolabe's card, then the draw step's; no second Astrolabe card"
    );
}

/// "{1}": the cost is a mana to pay, not just the tap and the sacrifice.
#[test]
fn astrolabe_cannot_be_activated_without_the_one_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[astrolabe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let astro = on_battlefield(&engine, p0, astrolabe()).unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: astro,
                    ability_index: 0,
                },
            )
            .is_err(),
        "no mana to pay {{1}} with"
    );
    assert!(on_battlefield(&engine, p0, astrolabe()).is_some());
    assert!(!is_tapped(&engine, astro));
}
