//! `cards/creatures/mv_7/howling_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn howling_giant() -> CardIndex {
    card_index("c616242a-95f0-4cd8-99a1-9237cfea0e57")
}

/// Howling Giant — a 5/5 with reach for {5}{G}{G} that makes two 2/2 Wolves.
#[test]
fn howling_giant_makes_two_wolves() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(forest(), 7), &[howling_giant()], &[]);
    let giant = cast_and_resolve(&mut engine, howling_giant());
    assert_body(&engine, giant, (5, 5), KeywordSet::REACH);
    let wolves = tokens_of(&engine, p0);
    assert_eq!(wolves.len(), 2);
    for wolf in wolves {
        assert_eq!(pt(&engine, wolf), (2, 2));
    }
}

/// Reach: the Giant is offered the block on a flier; a Gray Ogre is not.
#[test]
fn howling_giant_blocks_a_flier() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[howling_giant(), gray_ogre()])
        .battlefield(1, &[flier()])
        .start();
    keep_mulligans(&mut engine);
    let giant = on_battlefield(&engine, p0, howling_giant()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    let sphinx = on_battlefield(&engine, p1, flier()).expect("out");
    let blocks = their_attack_blocks(&mut engine, sphinx);
    let can = |who: ObjectId| {
        blocks
            .iter()
            .any(|b| b.blocker == who && b.attackers.contains(&sphinx))
    };
    assert!(can(giant), "reach");
    assert!(!can(ogre), "the Ogre cannot");
}
