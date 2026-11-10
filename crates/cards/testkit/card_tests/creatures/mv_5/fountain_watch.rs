//! `cards/creatures/mv_5/fountain_watch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "e6527ba3-3293-4a29-a2f9-7a81f8f27b7c"

/// Fountain Watch prints one static: "Artifacts and enchantments you control
/// have shroud." Both halves of that filter and both halves of "you control"
/// need a witness, so the board carries a Sol Ring and an Exploration of mine
/// (one artifact, one enchantment), an Elf of mine and a Sol Ring across the
/// table as the negatives, and the Watch itself — which grants the keyword
/// without keeping it, being a creature. The keyword is read off the
/// projection first, and then the sentence is *played*: the opponent's
/// Vindicate enumerates its legal targets, and neither of my shrouded
/// permanents may be on that list while my Elf, their own Sol Ring and the
/// Watch are — which tells shroud from a blanket refusal of my whole board and
/// from a filter that reads only one of the two types.
#[test]
fn fountain_watch_shrouds_my_artifacts_and_enchantments_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fountain_watch(),
                quiet_artifact(),
                exploration(),
                llanowar_elves(),
                plains(),
                plains(),
                swamp(),
            ],
        )
        // The artifact across the table, so "you control" is read rather than
        // assumed, and the mana for the Vindicate the other seat will cast.
        .battlefield(1, &[quiet_artifact(), plains(), swamp(), swamp()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let watch = on_battlefield(&engine, p0, fountain_watch()).expect("the Watch is out");
    let my_rock = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let my_chant = on_battlefield(&engine, p0, exploration()).expect("my Exploration is out");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // The two halves of the filter, read through the layers.
    assert!(
        keywords(&engine, my_rock).contains(KeywordSet::SHROUD),
        "an artifact this seat controls has shroud"
    );
    assert!(
        keywords(&engine, my_chant).contains(KeywordSet::SHROUD),
        "and so does an enchantment this seat controls"
    );
    // And the three permanents that are neither: a creature is none of the two
    // types, the artifact across the table is not `you control`, and the Watch
    // grants the keyword without keeping it.
    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::SHROUD),
        "a creature is neither an artifact nor an enchantment"
    );
    assert!(
        !keywords(&engine, their_rock).contains(KeywordSet::SHROUD),
        "\"you control\" is not \"the table\""
    );
    assert!(
        !keywords(&engine, watch).contains(KeywordSet::SHROUD),
        "the Watch grants the keyword, it does not keep it"
    );

    // The sentence itself, played from the far side of the table: shroud
    // refuses every player's spells, the controller's included, so the probe
    // is a Vindicate aimed at my board by the seat across it.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert!(
        options.contains(&my_elf),
        "a creature of mine carries no shroud and is on the menu: {options:?}"
    );
    assert!(
        options.contains(&their_rock),
        "and neither does their own Sol Ring: {options:?}"
    );
    assert!(
        options.contains(&watch),
        "nor the Watch itself, which is a creature and grants the keyword to \
         nobody's else's permanents: {options:?}"
    );
    assert!(
        !options.contains(&my_rock),
        "shroud: my Sol Ring is an artifact I control and cannot be the target \
         of anything: {options:?}"
    );
    assert!(
        !options.contains(&my_chant),
        "and the same for my enchantment, which is the other half of the \
         printed filter: {options:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .expect("the Elf was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Vindicate resolved against the creature it was allowed to name"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some()
            && on_battlefield(&engine, p0, exploration()).is_some(),
        "and the two permanents it could never name are still standing"
    );
}
