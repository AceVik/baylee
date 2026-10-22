//! `cards/artifacts/mv_4/bone_mask.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BONE_MASK, PRODIGAL_SORCERER};

#[test]
fn bone_mask_prevents_next_damage_from_chosen_source() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[BONE_MASK, plains(), plains()])
        .battlefield(1, &[PRODIGAL_SORCERER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let prodigal = on_battlefield(&engine, p1, PRODIGAL_SORCERER).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, BONE_MASK, 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseDamageSource, got {:?}", engine.pending());
    };
    let source = *options
        .iter()
        .find(|s| s.object == prodigal)
        .expect("Prodigal Sorcerer is offered as a damage source");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Prodigal Sorcerer pings p0 for 1 damage, which Bone Mask prevents.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, p1, PRODIGAL_SORCERER, 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "Bone Mask should prevent the next damage from the chosen source"
    );
}
