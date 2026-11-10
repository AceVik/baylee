//! `cards/enchantments/mv_5/screams_of_the_damned.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn screams_of_the_damned() -> CardIndex {
    card_index("831bb507-aaf0-4e6a-98d9-f010f96aff96")
}

/// "{1}{B}, Exile a card from your graveyard: This enchantment deals 1
/// damage to each creature and each player." The card in the graveyard is
/// exiled as the cost; the Elves die, the Ogre takes a point, and each
/// player loses 1.
#[test]
fn screams_of_the_damned_exiles_a_card_for_one_damage_to_everything() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[screams_of_the_damned(), swamp(), swamp(), gray_ogre()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    let graveyard =
        |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(graveyard(&engine), 1);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, screams_of_the_damned(), 0);
    let rest = drive_to_rest(&mut engine, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert_eq!(graveyard(&engine), 0, "the card was exiled as the cost");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "1 damage kills"
    );
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("2/2 survives 1");
    assert_eq!(engine.state().object(ogre).unwrap().damage, 1);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
    assert!(
        on_battlefield(&engine, p0, screams_of_the_damned()).is_some(),
        "it is not sacrificed"
    );
}
