//! `cards/creatures/mv_7/archweaver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn archweaver() -> CardIndex {
    card_index("2ceb76e2-1692-48fc-ab20-9587d1374172")
}

/// Archweaver — a 5/5 with reach and trample for {5}{G}{G}.
#[test]
fn archweaver_costs_seven_and_is_a_five_five_reach_trampler() {
    let mut engine = table7(&lots(forest(), 7), &[archweaver()], &[]);
    let spider = cast_and_resolve(&mut engine, archweaver());
    assert_body(
        &engine,
        spider,
        (5, 5),
        KeywordSet::REACH.union(KeywordSet::TRAMPLE),
    );
}

/// Trample: blocked by a 2/2, the Spider assigns 2 to it and 3 to the player.
#[test]
fn archweaver_tramples_over_a_blocker() {
    let (mut engine, spider, ogre) = duel_vs_ogre(archweaver(), &[]);
    fight(&mut engine, &[spider], &[(ogre, spider)]);
    assert!(
        in_graveyard(&engine, PlayerId::new(1), gray_ogre()).is_some(),
        "the Ogre took lethal damage"
    );
    assert_eq!(life_of(&engine, 1), 17, "the other 3 went to the player");
}

/// Reach: the Spider is offered the block on a flier, a Gray Ogre beside it
/// is not.
#[test]
fn archweaver_blocks_a_flier() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[archweaver(), gray_ogre()])
        .battlefield(1, &[flier()])
        .start();
    keep_mulligans(&mut engine);
    let spider = on_battlefield(&engine, PlayerId::new(0), archweaver()).expect("out");
    let ogre = on_battlefield(&engine, PlayerId::new(0), gray_ogre()).expect("out");
    let sphinx = on_battlefield(&engine, p1, flier()).expect("out");
    let blocks = their_attack_blocks(&mut engine, sphinx);
    let can = |who: ObjectId| {
        blocks
            .iter()
            .any(|b| b.blocker == who && b.attackers.contains(&sphinx))
    };
    assert!(can(spider), "reach lets the Spider block the flier");
    assert!(!can(ogre), "the Ogre has nothing to reach with");
}
