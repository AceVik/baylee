//! `cards/creatures/mv_2/oracle_en_vec.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn oracle_en_vec() -> CardIndex {
    card_index("367faa8c-617a-41a0-901c-f65106e86d56")
}

/// Oracle en-Vec (`367faa8c-617a-41a0-901c-f65106e86d56`) — {1}{W} Creature — Human Wizard (Partial):
/// "{T}: Target opponent chooses any number of creatures they control. During that player's next turn, the chosen creatures attack if able..."
#[test]
fn oracle_en_vec_forces_opponents_creatures_to_attack_on_their_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[oracle_en_vec()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    activate(&mut engine, P0, oracle_en_vec(), 0);
    legends_kit::aim(&mut engine, &[], &[P1]);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
}
