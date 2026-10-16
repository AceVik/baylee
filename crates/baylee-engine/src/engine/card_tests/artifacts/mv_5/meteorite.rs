//! `cards/artifacts/mv_5/meteorite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn meteorite() -> CardIndex {
    card_index("0c3c3bcc-d485-4a01-92fe-8bf29c0fc926")
}

/// "When this artifact enters, it deals 2 damage to any target." A player
/// can be the target.
#[test]
fn entering_deals_two_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[meteorite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, meteorite());
    pass_until_targets(&mut engine, p0);
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
    assert_eq!(life_of(&engine, p1), 18);
    assert_eq!(life_of(&engine, p0), 20);
}

/// The same trigger at a creature.
#[test]
fn the_damage_can_go_to_a_creature_instead() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[meteorite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, meteorite());
    pass_until_targets(&mut engine, p0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some(), "2 damage kills a 2/2");
    assert_eq!(life_of(&engine, p1), 20);
}

/// "{T}: Add one mana of any color."
#[test]
fn tapping_adds_one_mana_of_the_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[meteorite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rock = on_battlefield(&engine, p0, meteorite()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: rock })
        .expect("offered");
    while matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
            .expect("a listed color");
    }
    assert!(is_tapped(&engine, rock));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!((pool.available(ManaColor::Red), pool.total()), (1, 1));
}
