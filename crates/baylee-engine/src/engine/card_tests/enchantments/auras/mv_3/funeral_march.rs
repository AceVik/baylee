//! Funeral March — `{1}{B}{B}` Enchantment — Aura (Homelands)
//!
//! Enchant creature
//! When enchanted creature leaves the battlefield, its controller sacrifices a
//! creature of their choice.

#[allow(clippy::wildcard_imports)]
use super::*;

fn funeral_march() -> CardIndex {
    card_index("4d8afa5c-cef3-4e67-bd4c-ebd5c68b96a7")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn funeral_march_makes_enchanted_creatures_controller_sacrifice_a_creature_when_it_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[funeral_march(), lightning_bolt()])
        .battlefield(0, &[swamp(), swamp(), swamp(), mountain()])
        .battlefield(1, &[bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let march_hand = in_hand(&engine, p0, funeral_march()).unwrap();
    let bolt_hand = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let p1_bears = on_battlefield(&engine, p1, bears()).unwrap();
    let p1_ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: march_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Destroy the enchanted Grizzly Bears with Lightning Bolt: Funeral March triggers and forces p1 to sacrifice their remaining creature (Gray Ogre).
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_ogre],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(p1_bears).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(p1_ogre).unwrap().zone,
        Zone::Graveyard,
        "enchanted creature's controller sacrificed their remaining creature when enchanted creature left the battlefield"
    );
}
