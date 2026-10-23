//! `cards/creatures/mv_3/tempting_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tempting_licid() -> CardIndex {
    card_index("3b2206ed-55e9-4735-b969-59d9e089bb10")
}

/// Tempting Licid (`3b2206ed-55e9-4735-b969-59d9e089bb10`) — {2}{G} 2/2 Creature — Licid (Partial):
/// "{G}, {T}: All creatures able to block target creature this turn do so."
#[test]
fn tempting_licid_forces_all_able_creatures_to_block_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tempting_licid(), index::HILL_GIANT, forest()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, tempting_licid(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "able blocker must block the creature targeted by Tempting Licid"
    );
}
