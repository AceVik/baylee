//! `cards/creatures/artifacts/mv_1/lightning_core_excavator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn lightning_core_excavator() -> CardIndex {
    card_index("cdaf3711-5bec-41bf-b333-c5f34ccb12ef")
}

/// "{5}, {T}, Sacrifice this creature: It deals 3 damage to any target." At
/// a player: three life, the Excavator in the graveyard, and the ability
/// absent with only four mana.
#[test]
fn lightning_core_excavator_deals_three_to_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[lightning_core_excavator()], &[]);
    let me = on_battlefield(&engine, p0, lightning_core_excavator()).expect("seated");
    let life = engine.state().players[1].life;

    float_mana(&mut engine, ManaColor::Colorless, 4);
    assert!(!is_offered(&engine, me, 0), "{{5}} is five");
    float_mana(&mut engine, ManaColor::Colorless, 1);
    activate(&mut engine, p0, lightning_core_excavator(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target, got {:?}", engine.pending())
    };
    assert!(
        !options.is_empty(),
        "creatures are on the menu beside players"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, life - 3);
    assert!(in_graveyard(&engine, p0, lightning_core_excavator()).is_some());
}

/// The same ability at a creature: three damage kills the 2/2 Ogre.
#[test]
fn lightning_core_excavator_can_kill_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[lightning_core_excavator()], &[gray_ogre()]);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");
    assert!(pt(&engine, ogre).1 <= 3, "three damage is lethal to it");
    float_mana(&mut engine, ManaColor::Colorless, 5);
    activate(&mut engine, p0, lightning_core_excavator(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
}
