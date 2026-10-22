//! `cards/artifacts/mv_4/helm_of_possession.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn helm_of_possession() -> CardIndex {
    card_index("78f22413-4e61-4191-9d30-7b5d6538446e")
}

/// Helm of Possession (`78f22413-4e61-4191-9d30-7b5d6538446e`) — {4} Artifact:
/// "You may choose not to untap this artifact during your untap step. {2}, {T}, Sacrifice a creature: Gain control of target creature for as long as you control this artifact and this artifact remains tapped."
#[test]
fn helm_of_possession_sacrifices_creature_to_steal_target_while_tapped() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[helm_of_possession(), forest(), forest(), llanowar_elves()],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    let elves = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, helm_of_possession(), 1);
    aim_at(&mut engine, P0, giant);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, llanowar_elves()).is_none());
    assert_eq!(engine.state().object(giant).unwrap().controller, P0);
}
