//! `cards/creatures/mv_4/ballista_squad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5973725c-6e1f-4d7e-bcef-3d867ea4c244"
fn ballista_squad() -> CardIndex {
    card_index("5973725c-6e1f-4d7e-bcef-3d867ea4c244")
}

/// Ballista Squad ({3}{W} 2/2 Creature — Human Rebel):
/// "{X}{W}, {T}: This creature deals X damage to target attacking or blocking creature."
#[test]
fn ballista_squad_deals_x_damage_to_target_attacking_creature() {
    let mut engine = lk::game(
        SEED,
        &[ballista_squad()],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut engine, P1);
    lk::attack(&mut engine, &[giant]);

    // Before declaring blockers or after empty blockers: declare no blockers, then P0 activates.
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
    );
    activate(&mut engine, P0, ballista_squad(), 0);
    lk::drive(&mut engine, &[giant], true, 3);
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::HILL_GIANT).is_some(),
        "3 damage from Ballista Squad destroyed the 3/3 attacking Hill Giant"
    );
    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "non-attacking Grizzly Bears was untouched"
    );
    let _ = bears;
}
