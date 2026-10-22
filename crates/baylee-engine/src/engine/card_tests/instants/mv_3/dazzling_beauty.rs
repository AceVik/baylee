//! `cards/instants/mv_3/dazzling_beauty.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DAZZLING_BEAUTY, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Dazzling Beauty (de96ef93-a2e0-409d-9d25-fcb2405039b5) — {2}{W} Instant:
/// "Cast this spell only during the declare blockers step. Target unblocked
/// attacking creature becomes blocked. Draw a card at the beginning of the
/// next turn's upkeep."
#[test]
fn dazzling_beauty_stops_unblocked_attacker_combat_damage_and_draws_next_upkeep() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[DAZZLING_BEAUTY])
        .start();
    keep_mulligans(&mut engine);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // P1 gets priority in DeclareBlockers first; pass to P0.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, DAZZLING_BEAUTY);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // Hill Giant assigned no combat damage.
    assert_eq!(engine.state().players[0].life, 20);
}
