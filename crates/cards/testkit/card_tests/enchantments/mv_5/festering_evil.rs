//! `cards/enchantments/mv_5/festering_evil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn festering_evil() -> CardIndex {
    card_index("0581b047-ffa1-42cc-a175-b7bf928e279e")
}

/// "At the beginning of your upkeep, this enchantment deals 1 damage to each
/// creature and each player." Both players lose 1, both Elves die, and the
/// 2/2 Ogres only take a point of damage.
#[test]
fn festering_evil_pings_every_creature_and_player_in_my_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[festering_evil(), llanowar_elves(), gray_ogre()])
        .battlefield(1, &[llanowar_elves(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre survives");
    assert_eq!(engine.state().object(ogre).unwrap().damage, 1);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}

/// "{B}{B}, Sacrifice this enchantment: It deals 3 damage to each creature
/// and each player."
#[test]
fn festering_evil_sacrifices_for_three_damage_to_everything() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[festering_evil(), swamp(), swamp(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (life0, life1) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, festering_evil(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, festering_evil()).is_none(),
        "sacrificed"
    );
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert_eq!(engine.state().players[0].life, life0 - 3);
    assert_eq!(engine.state().players[1].life, life1 - 3);
}
