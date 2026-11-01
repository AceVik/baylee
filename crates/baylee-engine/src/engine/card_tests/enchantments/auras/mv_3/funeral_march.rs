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

/// "When enchanted creature leaves the battlefield": another creature dying
/// does nothing.
#[test]
fn funeral_march_does_nothing_when_a_creature_that_is_not_enchanted_dies() {
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

    // The Bolt kills the Ogre, which is not enchanted: no sacrifice is asked.
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(p1_ogre).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(p1_bears).unwrap().zone,
        Zone::Battlefield,
        "the enchanted creature is still there, and nothing was sacrificed"
    );
}

/// "Its controller sacrifices a creature of their choice": the controller of
/// the enchanted creature, not the Aura's, and from their own creatures.
#[test]
fn funeral_march_makes_its_own_controller_sacrifice_when_that_player_owns_the_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[funeral_march(), lightning_bolt()])
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), mountain(), bears(), gray_ogre()],
        )
        .battlefield(1, &[bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let march_hand = in_hand(&engine, p0, funeral_march()).unwrap();
    let bolt_hand = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let my_bears = on_battlefield(&engine, p0, bears()).unwrap();
    let my_ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let their_bears = on_battlefield(&engine, p1, bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: march_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the enchanted creature's controller chooses");
    assert!(options.contains(&my_ogre));
    assert!(
        !options.contains(&their_bears),
        "only their own creatures are on the menu"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_ogre],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(my_ogre).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(their_bears).unwrap().zone,
        Zone::Battlefield
    );
}
