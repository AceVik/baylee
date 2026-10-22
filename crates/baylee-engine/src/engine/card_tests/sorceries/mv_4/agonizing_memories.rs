//! `cards/sorceries/mv_4/agonizing_memories.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn agonizing_memories() -> CardIndex {
    card_index("2ac9d397-10ca-43ce-bff9-45d631553486")
}

/// `Agonizing Memories` ({2}{B}{B}, Sorcery, `Coverage::Partial`) looks at
/// target player's hand.
#[test]
fn agonizing_memories_looks_at_target_players_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[agonizing_memories()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, agonizing_memories());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));

    assert!(in_graveyard(&engine, p0, agonizing_memories()).is_some());
}
