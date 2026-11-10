//! `cards/enchantments/mv_3/hanna_s_custody.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hanna's Custody — {2}{W} enchantment: "All artifacts have shroud."
///
/// Shroud (CR 702.18) is only ever visible in a targeting question, and the
/// board carries two untapped Liquimetal Coatings — "{T}: Target permanent
/// becomes an artifact in addition to its other types" — precisely because
/// that ability may name an artifact or a land, so the same menu reads the
/// card twice: once before the enchantment arrives, with both Sol Rings, both
/// Coatings and an Elf on it, and once after, with a Forest and the
/// enchantment itself still on it while every artifact is gone — including the
/// Elf the first activation had turned into one. That the ability belongs to
/// the artifacts' own controller makes the absence shroud and not hexproof
/// (CR 702.11b), which would have left their owner free to aim at them.
#[test]
#[allow(clippy::too_many_lines)] // one menu, asked on both sides of the enchantment
fn hannas_custody_shrouds_every_artifact_on_the_table_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                liquimetal_coating(),
                liquimetal_coating(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[forest(), quiet_artifact()])
        .hand(0, &[hannas_custody()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: which seat the seed
    // puts on the play is not this test's subject, and only the tolerant
    // walker crosses a turn boundary to reach p0's own main phase.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let coatings = all_on_battlefield(&engine, p0, liquimetal_coating());
    assert_eq!(
        coatings.len(),
        2,
        "two copies, one per side of the enchantment"
    );
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");

    // The menu *before* the enchantment, so that its losses later are the
    // shroud and not this ability's own filter or its price.
    activate(&mut engine, p0, liquimetal_coating(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated aims it");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        [ring, theirs, coatings[0], coatings[1]]
            .iter()
            .all(|id| options.contains(id)),
        "with no shroud anywhere yet, artifacts sit on this menu like anything \
         else: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        types(&engine, elf).contains(TypeSet::ARTIFACT),
        "the Coating's own effect turned the Elf into an artifact in addition \
         to its other types"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::SHROUD),
        "and nothing on the board has granted shroud yet"
    );
    assert_eq!(
        coatings
            .iter()
            .filter(|id| is_tapped(&engine, **id))
            .count(),
        1,
        "the Coating whose {{T}} paid is the one left down; the other is the \
         reading this test still has to take"
    );

    // {2}{W} off the three Plains — and the Elf the first activation turned
    // into an artifact is still one, which is the point of the pair of menus.
    cast_from_hand(&mut engine, p0, hannas_custody());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let custody = on_battlefield(&engine, p0, hannas_custody()).expect("the Custody resolved");

    assert!(
        keywords(&engine, ring).contains(KeywordSet::SHROUD),
        "all artifacts have shroud"
    );
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::SHROUD),
        "on both sides of the table, and not only the controller's"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::SHROUD),
        "and so has the Elf the Coating made an artifact a moment earlier"
    );
    for coating in &coatings {
        assert!(
            keywords(&engine, *coating).contains(KeywordSet::SHROUD),
            "both Coatings are artifacts and neither is exempt"
        );
    }
    assert!(
        !keywords(&engine, custody).contains(KeywordSet::SHROUD),
        "the enchantment that grants the keyword is no artifact and keeps none"
    );
    assert!(
        !keywords(&engine, their_land).contains(KeywordSet::SHROUD),
        "and a Forest is not one either"
    );

    // The same menu again, from the Coating that has not spent its {T}.
    activate(&mut engine, p0, liquimetal_coating(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated aims it");
    assert!(
        options.contains(&their_land) && options.contains(&custody),
        "the menu is not empty: a Forest across the table and the enchantment \
         itself are still legal \"target permanent\"s: {options:?}"
    );
    for artifact in [ring, theirs, elf, coatings[0], coatings[1]] {
        assert!(
            !options.contains(&artifact),
            "every artifact on the table is off the menu now, whoever owns it: \
             {options:?}"
        );
    }

    // The answer the question offered still lands, and what becomes an
    // artifact after the enchantment is shrouded by it too: the static reads
    // the board it is on rather than a snapshot of it.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the Forest was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        types(&engine, their_land).contains(TypeSet::ARTIFACT),
        "the Coating makes the Forest an artifact in addition to its types"
    );
    assert!(
        keywords(&engine, their_land).contains(KeywordSet::SHROUD),
        "and Hanna's Custody shrouds what became an artifact after it resolved"
    );
}
