//! `cards/enchantments/mv_5/liliana_s_mastery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn liliana_s_mastery() -> CardIndex {
    card_index("bd104c7e-311e-4b03-98d3-5f20f3a99d26")
}

/// "When this enchantment enters, create two 2/2 black Zombie creature
/// tokens." "Zombies you control get +1/+1": the tokens are 3/3, and a
/// creature that is not a Zombie is not pumped.
#[test]
fn liliana_s_mastery_makes_two_pumped_zombies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[liliana_s_mastery()])
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), swamp(), gray_ogre()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    assert!(tokens_of(&engine, p0).is_empty());

    cast_from_hand(&mut engine, p0, liliana_s_mastery());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, liliana_s_mastery()).is_some()
    });
    let zombies = tokens_of(&engine, p0);
    assert_eq!(zombies.len(), 2, "two tokens");
    for z in zombies {
        assert_eq!(pt(&engine, z), (3, 3), "2/2 plus the anthem");
    }
    assert_eq!(pt(&engine, ogre), (2, 2), "an Ogre is not a Zombie");
}
