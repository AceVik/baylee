//! `cards/enchantments/auras/mv_3/spectral_shield.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spectral_shield() -> CardIndex {
    card_index("37d9c065-598d-4ee3-abfc-625bd6169ff1")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

/// Spectral Shield (37d9c065-598d-4ee3-abfc-625bd6169ff1) — {1}{W}{U} Enchantment — Aura:
/// - "Enchant creature"
/// - "Enchanted creature gets +0/+2 and can't be the target of spells."
#[test]
fn spectral_shield_gives_plus_zero_plus_two_and_prevents_spells_but_not_abilities_from_targeting() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), island(), mountain(), grizzly_bears(), gray_ogre()],
        )
        .battlefield(1, &[mountain(), prodigal_sorcerer()])
        .hand(0, &[spectral_shield()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();

    // Cast Spectral Shield on Grizzly Bears -> becomes 2/4.
    cast_from_hand(&mut engine, P0, spectral_shield());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (2, 4));

    // 1. P1 casts a spell (Lightning Bolt): enchanted Grizzly Bears is NOT in the target menu,
    // while unenchanted Gray Ogre IS in the target menu.
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Lightning Bolt");
    };
    assert!(
        !options.contains(&bears),
        "enchanted creature can't be the target of spells"
    );
    assert!(
        options.contains(&ogre),
        "unenchanted creature can still be targeted by spells"
    );
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 2. P1 activates an ability (Prodigal Sorcerer): enchanted Grizzly Bears CAN be targeted by abilities!
    activate(&mut engine, P1, prodigal_sorcerer(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Prodigal Sorcerer");
    };
    assert!(
        options.contains(&bears),
        "enchanted creature CAN be targeted by activated abilities"
    );
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        1,
        "Prodigal Sorcerer's ability dealt 1 damage to the enchanted creature"
    );
}
