//! Touch of Death — `{2}{B}` Sorcery (Ice Age)
//!
//! Touch of Death deals 1 damage to target player or planeswalker and you gain
//! 1 life.
//! Draw a card at the beginning of the next turn's upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn touch_of_death() -> CardIndex {
    card_index("52a768eb-16eb-4643-b8be-7a4b5b32f28b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn touch_of_death_drains_one_life_from_target_player_and_draws_on_next_turns_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[touch_of_death()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, touch_of_death()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    // Only players (or planeswalkers) are legal targets; creatures are excluded.
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        options.is_empty(),
        "creatures cannot be targeted by Touch of Death"
    );
    assert!(player_options.contains(&p1));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 21);
    let hand_after_resolve = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Advance to p1's upkeep: delayed trigger draws 1 card for p0.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_resolve + 1,
        "p0 draws a card at the beginning of the next turn's upkeep"
    );
}
