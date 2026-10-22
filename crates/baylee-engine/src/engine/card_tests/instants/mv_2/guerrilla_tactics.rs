//! Guerrilla Tactics — `{1}{R}` Instant (Alliances)
//!
//! Guerrilla Tactics deals 2 damage to any target.
//! When a spell or ability an opponent controls causes you to discard this card,
//! it deals 4 damage to any target.

#[allow(clippy::wildcard_imports)]
use super::*;

fn guerrilla_tactics() -> CardIndex {
    card_index("65ad90f5-4dae-494b-83fb-8052e77e0b12")
}

fn thoughtseize() -> CardIndex {
    baylee_core::generated::index::THOUGHTSEIZE
}

#[test]
fn guerrilla_tactics_cast_deals_2_damage_and_discarded_by_opponent_deals_4_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[guerrilla_tactics(), guerrilla_tactics()])
        .battlefield(0, &[mountain(), mountain()])
        .hand(1, &[thoughtseize()])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. Cast Guerrilla Tactics normally -> deals 2 damage to p1.
    let gt1 = in_hand(&engine, p0, guerrilla_tactics()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gt1 })
        .unwrap();
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

    assert_eq!(engine.state().players[1].life, 18);

    // 2. Advance to p1's main phase. p1 casts Thoughtseize targeting p0, discarding the second Guerrilla Tactics.
    reach_their_main_phase(&mut engine, p1);
    let ts = in_hand(&engine, p1, thoughtseize()).unwrap();
    let gt2 = in_hand(&engine, p0, guerrilla_tactics()).unwrap();

    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: ts })
        .unwrap();
    engine.apply(p1, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![gt2] })
        .unwrap();

    // Thoughtseize also makes p1 lose 2 life (to 16), and Guerrilla Tactics's DiscardedThisByOpponent trigger asks p0 for any target.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
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

    assert_eq!(
        engine.state().players[1].life,
        12,
        "18 - 2 (Thoughtseize) - 4 (Guerrilla Tactics discard trigger) = 12"
    );
}
