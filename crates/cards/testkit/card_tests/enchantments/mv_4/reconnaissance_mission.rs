//! `cards/enchantments/mv_4/reconnaissance_mission.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn reconnaissance_mission() -> CardIndex {
    card_index("11bf211c-95a3-4e7f-bd08-13f979d54e73")
}

/// "Whenever a creature you control deals combat damage to a player, you may
/// draw a card.": asked after the damage, and yes draws.
#[test]
fn reconnaissance_mission_draws_when_my_creature_connects() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[reconnaissance_mission(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    attack_with(&mut engine, bears, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert_eq!(life_of(&engine, p1), 18, "the damage came first");
    let library = library_size(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1, "one card drawn");
}

/// "Cycling {2}": from the hand, for {2}, the card goes to the graveyard and
/// its owner draws a card.
#[test]
fn reconnaissance_mission_cycles_from_hand_for_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[reconnaissance_mission()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library = library_size(&engine, p0);

    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|&(id, _)| in_hand(&engine, p0, reconnaissance_mission()) == Some(id)),
        "cycling costs {{2}} and none is available"
    );
    float(&mut engine, p0, ManaColor::Green, 2);
    activate(&mut engine, p0, reconnaissance_mission(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, reconnaissance_mission()).is_some(),
        "discarded as the cost"
    );
    assert_eq!(library_size(&engine, p0), library - 1, "and a card drawn");
}
