//! `cards/creatures/mv_5/ambush_commander.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn ambush_commander_animates_forests_as_elves_and_sacrifices_an_elf_to_pump() {
    let commander = card_index("e75b2a97-b28d-43fe-8b1e-ba87a09f339c");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[commander, forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cmd_id = on_battlefield(&engine, p0, commander).unwrap();
    let forests = lands_of(&engine, p0);
    assert_eq!(forests.len(), 3);
    for &f in &forests {
        assert_eq!(pt(&engine, f), (1, 1));
        let chars = engine.state().object(f).unwrap().characteristics().clone();
        assert!(chars.types.contains(TypeSet::CREATURE));
        assert!(chars.types.contains(TypeSet::LAND));
        assert!(
            chars
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::ELF)
        );
    }

    // Tap two Forests for {G}{G}, then sacrifice the third Forest (which is an Elf!) to pump Ambush Commander +3/+3.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forests[0] })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forests[1] })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cmd_id,
                ability_index: 4,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![cmd_id],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forests[2]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, cmd_id), (5, 5));
    assert_eq!(lands_of(&engine, p0).len(), 2);
}
