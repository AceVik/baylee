//! `cards/enchantments/mv_3/aura_of_silence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::ROD_OF_RUIN;

fn aura_of_silence() -> CardIndex {
    card_index("e7faf8eb-e829-4109-8dfe-42865a23ba86")
}

/// `Aura of Silence` ({1}{W}{W}, Enchantment) sacrifices itself to destroy
/// target artifact or enchantment.
#[test]
fn aura_of_silence_taxes_opponent_artifacts_and_sacrifices_to_destroy_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[ROD_OF_RUIN])
        .hand(0, &[aura_of_silence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aura_of_silence());
    pass_until(&mut engine, stack_is_empty);

    let rod = on_battlefield(&engine, p1, ROD_OF_RUIN).unwrap();
    activate(&mut engine, p0, aura_of_silence(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![rod],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, aura_of_silence()).is_some());
    assert!(in_graveyard(&engine, p1, ROD_OF_RUIN).is_some());
}
