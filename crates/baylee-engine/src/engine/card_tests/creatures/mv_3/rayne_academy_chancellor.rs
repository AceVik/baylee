//! `cards/creatures/mv_3/rayne_academy_chancellor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rayne_academy_chancellor() -> CardIndex {
    card_index("6933a7bf-10ed-46c8-a49c-ce200bce3eea")
}

/// Rayne, Academy Chancellor (6933a7bf-10ed-46c8-a49c-ce200bce3eea) — {2}{U}
/// 1/1 Legendary Creature — Human Wizard (Partial):
/// "Whenever you or a permanent you control becomes the target of a spell or
/// ability an opponent controls, you may draw a card. You may draw an
/// additional card if Rayne is enchanted."
#[test]
fn rayne_triggers_when_opponent_targets_you_or_your_permanent_and_ignores_own_targeting() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[rayne_academy_chancellor(), index::HILL_GIANT, mountain()],
        )
        .battlefield(1, &[mountain(), mountain(), index::HILL_GIANT])
        .hand(0, &[lightning_bolt()])
        .hand(1, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_giant = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();

    // 1. Negative boundary: P0 targeting their own permanent with their own spell does not trigger Rayne.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, my_giant);
    let after_own_cast = hand_size(&engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        after_own_cast,
        "targeting your own permanent with your own spell does not trigger Rayne"
    );

    // 2. Opponent targets P0 ("you") -> Rayne triggers MayDo; P0 accepts and draws 1 card.
    reach_their_main_phase(&mut engine, p1);
    let before_first_opp_bolt = hand_size(&engine, p0);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        before_first_opp_bolt + 1,
        "Rayne draws a card when an opponent's spell targets you"
    );

    // 3. Opponent targets Rayne (a permanent P0 controls) -> Rayne triggers MayDo; P0 declines and draws 0.
    let rayne = on_battlefield(&engine, p0, rayne_academy_chancellor()).unwrap();
    let before_second_opp_bolt = hand_size(&engine, p0);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    aim_at(&mut engine, p1, rayne);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        before_second_opp_bolt,
        "declining Rayne's MayDo draws no card"
    );
}
