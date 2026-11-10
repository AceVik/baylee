//! `cards/enchantments/mv_5/blockbuster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn blockbuster() -> CardIndex {
    card_index("11e50e35-bf4b-4089-b0c9-360d9c014584")
}

/// "{1}{R}, Sacrifice this enchantment: It deals 3 damage to each tapped
/// creature and each player." The tapped Elves (tapped for the mana) die;
/// the untapped Ogres do not; each player takes 3.
#[test]
fn blockbuster_hits_tapped_creatures_and_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[blockbuster(), mountain(), mountain(), llanowar_elves()],
        )
        .battlefield(1, &[gray_ogre(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    let their_ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    // Tap my Elves with the Mountains for mana; theirs stay untapped.
    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, my_elves));
    assert!(!is_tapped(&engine, their_elves));

    activate(&mut engine, p0, blockbuster(), 0);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, blockbuster()).is_none(),
        "sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, blockbuster()).is_some(),
        "and in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the tapped Elves took 3"
    );
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert_eq!(engine.state().object(their_ogre).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 17);
}
