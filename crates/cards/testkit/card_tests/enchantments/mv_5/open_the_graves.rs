//! `cards/enchantments/mv_5/open_the_graves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn open_the_graves() -> CardIndex {
    card_index("28778958-a1f9-4fea-b551-c193d1257f18")
}

/// "Whenever a nontoken creature you control dies, create a 2/2 black Zombie
/// creature token." Their creature: nothing. Mine: one Zombie. The Zombie
/// token itself dying: nothing more.
#[test]
fn open_the_graves_raises_a_zombie_for_a_nontoken_creature_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[open_the_graves(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    kill(&mut engine, theirs);
    assert!(tokens_of(&engine, p0).is_empty(), "not my creature");

    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    kill(&mut engine, mine);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Zombie");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));

    kill(&mut engine, tokens[0]);
    assert!(tokens_of(&engine, p0).is_empty(), "a token does not count");
}
