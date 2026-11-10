//! `cards/creatures/mv_5/plated_spider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plated Spider — {4}{G} — Creature — Spider, a printed 4/4 with reach:
/// "Reach (This creature can block creatures with flying.)"
///
/// Reach is a rule about the block declaration and nothing else, so a test
/// that only read the projected keyword would go on passing over a Spider the
/// engine never offered against a flier. The Spider is cast for real off five
/// Forests, an untapped Elf stands beside it as the control with neither reach
/// nor flying, and the opponent attacks with a flier: the pairing the engine
/// publishes in `Pending::ChooseBlockers` has to hold the Spider and the flier,
/// and must not hold the Elf and the flier.
#[test]
fn plated_spider_blocks_the_flier_the_elf_beside_it_may_not_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[sphinx_of_the_final_word()])
        .hand(0, &[plated_spider()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf is the control the whole test turns on, so it is named as the
    // one printing kept back: `tap_all_mana_but` leaves it standing, and a
    // creature tapped for mana is a creature that may not block.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests, five green, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, plated_spider());
    pass_until(&mut engine, stack_is_empty);

    let spider = on_battlefield(&engine, p0, plated_spider()).expect("the Spider resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    let flier = on_battlefield(&engine, p1, sphinx_of_the_final_word())
        .expect("the flying creature is on the table");
    assert_eq!(pt(&engine, spider), (4, 4), "the body the card prints");
    assert!(
        keywords(&engine, spider).contains(KeywordSet::REACH),
        "the printed reach reaches the permanent"
    );
    assert!(
        keywords(&engine, flier).contains(KeywordSet::FLYING),
        "and the creature across the table is a flier, which is the thing reach is for"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::REACH)
            && !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "the control has neither of the two keywords that reach a flier"
    );

    // The attack declaration is what publishes the pairing at all: nothing the
    // engine asks before it says what may block what.
    reach_their_main_phase(&mut engine, p1);
    let blocks = attack_and_collect_blocks(&mut engine, flier, p0);

    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == spider && b.attackers.contains(&flier)),
        "\"This creature can block creatures with flying\": the Spider is one of \
         the blockers offered against the flier, got {blocks:?}"
    );
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&flier)),
        "the Elf has no reach and no flying, so the flier is not a pairing it may \
         be given: {blocks:?}"
    );
}
