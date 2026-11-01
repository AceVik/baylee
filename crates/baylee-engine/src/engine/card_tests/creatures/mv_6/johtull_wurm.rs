//! `cards/creatures/mv_6/johtull_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn johtull_wurm() -> CardIndex {
    card_index("ab5a8242-4e42-43e6-a6ee-eeb0a8db3d11")
}

/// Johtull Wurm (ab5a8242-4e42-43e6-a6ee-eeb0a8db3d11) — {5}{G} 6/6 Creature — Wurm:
/// - "Whenever this creature becomes blocked, it gets -2/-1 until end of turn for each creature blocking it beyond the first."
#[test]
fn johtull_wurm_stays_six_six_against_one_blocker_and_shrinks_per_blocker_beyond_first() {
    let mut engine = lk::game(
        SEED,
        &[johtull_wurm()],
        &[
            index::WALL_OF_STONE,
            index::WALL_OF_WOOD,
            index::WALL_OF_ICE,
        ],
        &[],
        &[],
    );
    let wurm = lk::obj(&engine, P0, johtull_wurm());
    let stone = lk::obj(&engine, P1, index::WALL_OF_STONE);
    let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    let ice = lk::obj(&engine, P1, index::WALL_OF_ICE);

    // Combat 1: blocked by a single creature (0/8 Wall of Stone) -> 0 blockers beyond the first -> stays 6/6.
    let _ = lk::attack(&mut engine, &[wurm]);
    lk::block(&mut engine, &[(stone, wurm)]);
    // Resolve the BecomesBlocked trigger before combat damage.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        pt(&engine, wurm),
        (6, 6),
        "blocked by only 1 creature, Johtull Wurm gets -0/-0 and stays 6/6"
    );
    lk::through_combat(&mut engine);

    // Advance to P0's next main phase.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    // Combat 2: blocked by 3 creatures (2 blockers beyond the first) -> gets -4/-2 (becomes 2/4) until end of turn.
    let _ = lk::attack(&mut engine, &[wurm]);
    lk::block(&mut engine, &[(stone, wurm), (wood, wurm), (ice, wurm)]);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        pt(&engine, wurm),
        (2, 4),
        "blocked by 3 creatures (2 beyond the first), Johtull Wurm gets -4/-2 until end of turn"
    );
    // Assign the 2 combat damage across the 3 blockers and advance to P1's main phase.
    while !matches!(engine.state().turn.phase, Phase::SecondMain) {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, max, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(max))
                    .unwrap();
            }
            other => panic!("unexpected during combat: {other:?}"),
        }
    }

    // At end of turn, the -4/-2 wears off and Johtull Wurm returns to 6/6.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, wurm), (6, 6));
}
