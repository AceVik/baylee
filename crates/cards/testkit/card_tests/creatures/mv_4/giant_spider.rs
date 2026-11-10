//! `cards/creatures/mv_4/giant_spider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Giant Spider` is a 2/4 Spider costing `{3}{G}` under `Coverage::Implemented` with reach.
/// When defending against an opponent attacking with a flying creature (such as `Desert Drake`),
/// reach allows `Giant Spider` to be legally assigned as a blocker in `Pending::ChooseBlockers`,
/// whereas an ordinary ground creature without reach is refused.
#[test]
fn giant_spider_has_reach_and_blocks_flying_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[giant_spider(), llanowar_elves()])
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);

    let drake =
        on_battlefield(&engine, p1, desert_drake()).expect("opponent controls Desert Drake");
    let spider = on_battlefield(&engine, p0, giant_spider()).expect("p0 controls Giant Spider");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls Llanowar Elves");

    assert_eq!(pt(&engine, spider), (2, 4), "printed body is 2/4");
    assert!(
        keywords(&engine, spider).contains(KeywordSet::REACH),
        "Giant Spider has reach"
    );

    let blocks = attack_and_collect_blocks(&mut engine, drake, p0);

    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == spider && b.attackers.contains(&drake)),
        "reach allows Giant Spider to block flying Desert Drake: {blocks:?}"
    );
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&drake)),
        "Llanowar Elves lacks reach and cannot block flyer: {blocks:?}"
    );
}
