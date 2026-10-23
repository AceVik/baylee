//! `cards/creatures/mv_4/mogg_bombers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mogg_bombers() -> CardIndex {
    card_index("69812cd2-9885-45ed-9d79-658597bfeb8b")
}

/// Mogg Bombers (`69812cd2-9885-45ed-9d79-658597bfeb8b`) — {3}{R} 3/4 Creature — Goblin (Implemented):
/// "Whenever another creature enters, sacrifice this creature and it deals 3 damage to target player or planeswalker."
#[test]
fn mogg_bombers_sacrifices_itself_and_deals_three_damage_when_another_creature_enters() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mogg_bombers(), forest(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, mogg_bombers()).is_none());
    assert!(in_graveyard(&engine, P0, mogg_bombers()).is_some());
    assert_eq!(engine.state().players[1].life, 17);
}
