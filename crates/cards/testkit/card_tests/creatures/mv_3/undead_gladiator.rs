//! `cards/creatures/mv_3/undead_gladiator.rs`, played.
//!
//! Oracle:
//! - "{1}{B}, Discard a card: Return this card from your graveyard to your hand.
//!   Activate only during your upkeep."
//! - "Cycling {1}{B}"

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn undead_gladiator() -> CardIndex {
    card_index("4da2425d-2118-4116-855d-2ef80693a7b7")
}

#[test]
fn undead_gladiator_cycles_and_returns_only_during_your_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[undead_gladiator(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Cycle Undead Gladiator from hand for {1}{B}: moves to graveyard and draws 1 card.
    let swamps = all_on_battlefield(&engine, P0, swamp());
    for &s in &swamps[..2] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    activate(&mut engine, P0, undead_gladiator(), 0);
    pass_until(&mut engine, stack_is_empty);

    let glad_gy = in_graveyard(&engine, P0, undead_gladiator()).expect("cycled to graveyard");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "discarded 1, drew 1"
    );

    // Graveyard ability is NOT offered during main phase.
    assert!(!ability_offered(&engine, glad_gy, 1));

    // Graveyard ability is NOT offered during opponent's upkeep.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(!ability_offered(&engine, glad_gy, 1));

    // During own upkeep with mana + discard card in hand: ability 1 IS offered and returns it.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let swamps = all_on_battlefield(&engine, P0, swamp());
    for &s in &swamps[..2] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    assert!(ability_offered(&engine, glad_gy, 1));
    activate(&mut engine, P0, undead_gladiator(), 1);

    let Pending::ChooseCards {
        options,
        prompt: ChoicePrompt::CostDiscard,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostDiscard prompt, got {:?}", engine.pending());
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, undead_gladiator()).is_none());
    assert!(in_hand(&engine, P0, undead_gladiator()).is_some());

    // Cast returned Gladiator in FirstMain for {1}{B}{B} -> 3/1 on battlefield.
    assert!(walk_to_own_main(&mut engine, P0));
    cast_from_hand(&mut engine, P0, undead_gladiator());
    pass_until(&mut engine, stack_is_empty);
    let glad = on_battlefield(&engine, P0, undead_gladiator()).expect("Gladiator resolved");
    assert_eq!(pt(&engine, glad), (3, 1));
}
