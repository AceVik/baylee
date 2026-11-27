//! `cards/enchantments/mv_2/oversold_cemetery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn oversold_cemetery() -> CardIndex {
    card_index("ed4cd3a1-688b-4c05-948d-39d3336e00c0")
}

/// Oversold Cemetery's upkeep trigger does nothing with three creature cards in your graveyard.
#[test]
fn oversold_cemetery_does_not_trigger_below_four_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                oversold_cemetery(),
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
                index::PEARLED_UNICORN,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let to_bury = vec![
        on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap(),
        on_battlefield(&engine, p0, index::LLANOWAR_ELVES).unwrap(),
        on_battlefield(&engine, p0, index::PEARLED_UNICORN).unwrap(),
    ];
    bury(&mut engine, &to_bury);
    reach_main_phase(&mut engine, p0);
    advance_next_main(&mut engine, p1);
    advance_next_main(&mut engine, p0);
    assert!(in_hand(&engine, p0, index::GRIZZLY_BEARS).is_none());
}

/// With four or more creature cards in your graveyard on upkeep, Oversold Cemetery
/// optionally returns target creature card to your hand.
#[test]
fn oversold_cemetery_returns_creature_when_four_or_more_in_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                oversold_cemetery(),
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
                index::PEARLED_UNICORN,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let mut to_bury = all_on_battlefield(&engine, p0, index::GRIZZLY_BEARS);
    to_bury.push(on_battlefield(&engine, p0, index::LLANOWAR_ELVES).unwrap());
    to_bury.push(on_battlefield(&engine, p0, index::PEARLED_UNICORN).unwrap());
    bury(&mut engine, &to_bury);
    reach_main_phase(&mut engine, p0);
    advance_next_main(&mut engine, p1);
    loop {
        if engine.state().turn.active == p0
            && matches!(engine.pending(), Pending::ChooseTargets { .. })
        {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    let bear_gy = in_graveyard(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_gy],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    assert!(in_hand(&engine, p0, index::GRIZZLY_BEARS).is_some());
}

fn advance_next_main(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    let start_turn = engine.state().turn.number;
    loop {
        if engine.state().turn.number > start_turn
            && engine.state().turn.active == target
            && engine.state().turn.phase == Phase::FirstMain
            && at_rest(engine, target)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending while advancing: {other:?}"),
        }
    }
}
