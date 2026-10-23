//! `cards/enchantments/mv_3/invasion_plans.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn invasion_plans() -> CardIndex {
    card_index("db9c0d94-9fa5-4f07-be48-23e19d2f3d68")
}

/// Invasion Plans (`db9c0d94-9fa5-4f07-be48-23e19d2f3d68`) — {2}{R} Enchantment (Partial):
/// "All creatures block each combat if able."
#[test]
fn invasion_plans_requires_creatures_to_block_if_able() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[invasion_plans(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let attacker = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(
        engine
            .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err()
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, attacker)],
                },
            )
            .is_ok()
    );
}
