//! `cards/enchantments/auras/mv_4/living_terrain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

fn living_terrain() -> CardIndex {
    card_index("ffa40f10-4efb-48d8-b16e-8a5711fcfad1")
}

/// Living Terrain ({2}{G}{G} Enchantment — Aura):
/// "Enchant land"
/// "Enchanted land is a 5/6 green Treefolk creature that's still a land."
#[test]
fn living_terrain_animates_enchanted_land_as_5_6_green_treefolk_creature() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest(), forest(), forest(), forest()])
        .hand(0, &[living_terrain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land = on_battlefield(&engine, p0, swamp()).unwrap();
    cast_from_hand(&mut engine, p0, living_terrain());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let view = engine.state().object(target_land).unwrap();
    assert_eq!(view.characteristics().power, Some(5));
    assert_eq!(view.characteristics().toughness, Some(6));
    assert!(view.characteristics().types.contains(TypeSet::LAND));
    assert!(view.characteristics().types.contains(TypeSet::CREATURE));
    assert!(
        view.characteristics()
            .subtypes
            .contains(subtypes::creature::TREEFOLK)
    );
    assert_eq!(
        view.characteristics().colors,
        ColorSet::from_slice(&[Color::Green])
    );
}

/// When Living Terrain leaves the battlefield, the enchanted land ceases to be
/// a 5/6 creature and remains a normal land.
#[test]
fn living_terrain_leaving_battlefield_reverts_land_to_noncreature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest(), forest(), forest(), forest()])
        .hand(0, &[living_terrain()])
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[index::DISENCHANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land = on_battlefield(&engine, p0, swamp()).unwrap();
    cast_from_hand(&mut engine, p0, living_terrain());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, living_terrain()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, index::DISENCHANT);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![aura],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let view = engine.state().object(target_land).unwrap();
    assert!(view.characteristics().types.contains(TypeSet::LAND));
    assert!(!view.characteristics().types.contains(TypeSet::CREATURE));
}
