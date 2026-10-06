//! `cards/instants/mv_1/deathlace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deathlace: "Target spell or permanent becomes black."
#[test]
fn deathlace_turns_its_target_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), swamp()])
        .hand(0, &[deathlace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, deathlace());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("its own Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().colors,
        ColorSet::of(Color::Black),
        "\"becomes black\""
    );
}
