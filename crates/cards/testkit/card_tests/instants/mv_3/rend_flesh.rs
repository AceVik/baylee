//! `cards/instants/mv_3/rend_flesh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rend Flesh — {2}{B} Instant, Arcane: "Destroy target non-Spirit creature."
///
/// The word that carries the card is `non-Spirit`, and no reading of the card
/// file can tell that filter from a promise — the menu is the claim. So the
/// board puts a Llanowar Elves and a Kami of Tattered Shoji under the same
/// opponent: a creature and a Spirit, which is the one distinction the printed
/// sentence draws, and a Sol Ring beside them that is no creature at all.
///
/// The answer is then taken twice — once with the Spirit, refused without the
/// other seat paying for it, and once with the Elf — and the target is named
/// while both creatures are still standing (CR 601.2c), because the destruction
/// is the resolution and not the announcement.
///
/// The three Swamps pay the printed {2}{B} down to an empty pool, so the spell
/// was really cast rather than announced, and the graveyard reads the instant
/// itself as well as the creature it killed.
#[test]
fn rend_flesh_destroys_a_non_spirit_creature_and_leaves_a_spirit_standing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(
            1,
            &[llanowar_elves(), kami_of_tattered_shoji(), quiet_artifact()],
        )
        .hand(0, &[rend_flesh()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let spirit =
        on_battlefield(&engine, p1, kami_of_tattered_shoji()).expect("their Spirit is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 to kill");
    assert_eq!(pt(&engine, spirit), (2, 5), "and a printed 2/5 Spirit");

    cast_from_hand(&mut engine, p0, rend_flesh());

    let options = pass_until_targets(&mut engine, p0);
    // CR 601.2c takes the target before CR 601.2h pays, so while this question
    // stands the three Swamps' mana is still floating.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    assert!(
        options.contains(&elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&spirit),
        "\"non-Spirit\" is the whole filter: a Spirit is a creature and still \
         not a legal target for this spell: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "the Sol Ring is an artifact and no creature: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "CR 601.2c before the effect resolves: the creature is named while it \
         is still standing"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![spirit],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, kami_of_tattered_shoji()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered is the one that dies");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Swamps are exactly {{2}}{{B}}, so the cost was paid and not \
         merely printed"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "\"Destroy target … creature\": the Elf is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the destruction itself"
    );
    assert!(
        on_battlefield(&engine, p1, kami_of_tattered_shoji()).is_some(),
        "the Spirit the filter declined never moved: one target, one death"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the Sol Ring was never on the menu to begin with"
    );
    assert!(
        in_graveyard(&engine, p0, rend_flesh()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );
}
