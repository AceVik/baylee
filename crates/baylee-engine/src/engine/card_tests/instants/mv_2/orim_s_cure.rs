//! `cards/instants/mv_2/orim_s_cure.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "61eacaab-625b-499c-8602-f37ad5dda138"
fn orim_s_cure() -> CardIndex {
    card_index("61eacaab-625b-499c-8602-f37ad5dda138")
}

/// `Orim's Cure` (`Coverage::Partial`):
/// "Prevent the next 4 damage that would be dealt to any target this turn."
///
/// Verifies that `Orim's Cure` prevents the next 4 damage dealt to the targeted
/// player across two 3-damage `Lightning Bolt`s, leaving 2 damage dealt.
#[test]
fn orims_cure_prevents_next_four_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9416, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(0, &[orim_s_cure()])
        .hand(1, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, orim_s_cure());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // First Lightning Bolt (3 damage): all 3 prevented, 1 shield remains.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
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
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);

    // Second Lightning Bolt (3 damage): 1 prevented, 2 dealt -> life 18.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
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
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 18);
}
