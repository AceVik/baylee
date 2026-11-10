//! `cards/enchantments/mv_5/asceticism.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn asceticism() -> CardIndex {
    card_index("9396546c-d067-4ce3-9c3d-b62cca970b4f")
}

/// "Creatures you control have hexproof." Mine do; the opponent's do not.
#[test]
fn asceticism_gives_hexproof_to_my_creatures_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[asceticism(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    assert!(keywords(&engine, mine).contains(KeywordSet::HEXPROOF));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::HEXPROOF));
}

/// "{1}{G}: Regenerate target creature." The ability asks for a target,
/// puts a shield on it, and the shield saves it from destruction.
#[test]
fn asceticism_regenerates_the_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[asceticism(), forest(), forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, asceticism(), 1);
    let options = aim_at(&mut engine, p0, ogre);
    assert!(options.contains(&ogre), "any creature is a legal target");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().object(ogre).unwrap().regeneration_shields,
        1,
        "one shield on the target"
    );
    kill(&mut engine, ogre);
    let ogre_now = on_battlefield(&engine, p0, gray_ogre()).expect("it regenerated");
    assert!(is_tapped(&engine, ogre_now), "regeneration taps it");
    assert!(in_graveyard(&engine, p0, gray_ogre()).is_none());
}
