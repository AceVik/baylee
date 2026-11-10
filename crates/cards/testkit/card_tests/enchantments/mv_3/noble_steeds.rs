//! `cards/enchantments/mv_3/noble_steeds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Noble Steeds — {2}{W} enchantment: "{1}{W}: Target creature gains first
/// strike until end of turn."
///
/// The printed filter is `Filter::CREATURE` and not "a creature you control",
/// so the scenario seats a Llanowar Elves on each side of the table: the menu
/// has to name both, and only the one that was aimed at may carry the keyword
/// afterwards. The Steeds itself is an enchantment — no creature, and so not
/// on its own menu — and the turn walked at the end is what tells the printed
/// "until end of turn" from a grant the board would have kept.
#[test]
#[allow(clippy::too_many_lines)]
fn noble_steeds_grants_first_strike_to_the_creature_it_names_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[noble_steeds()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "nothing has granted anything yet"
    );

    // Five Plains into the pool, and the Elf named as the printing kept back:
    // it is the creature the ability is about to aim at, and a mana creature
    // tapped for the cost would read as a different board afterwards.
    tap_mana_except(&mut engine, p0, elf);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, noble_steeds());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let steeds = on_battlefield(&engine, p0, noble_steeds()).expect("the Steeds resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{2}}{{W}} is spent and exactly the {{1}}{{W}} the ability charges is left"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands — so the claim is made with the mana already
    // floating, which is where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(steeds, 0)),
        "with the mana floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, noble_steeds(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the ability asks once"
    );
    assert!(
        options.contains(&elf) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&steeds),
        "the Steeds is an enchantment and no creature: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2c before CR 601.2h: the target is named while the mana is still in the pool"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "the creature the ability named gained first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "the Elf nobody named never moved: the effect targets, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, steeds).contains(KeywordSet::FIRST_STRIKE),
        "the Steeds grants the keyword, it does not keep it"
    );

    // "until end of turn": one turn later the Elf is a printed 1/1 again, so
    // the keyword was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the keyword left rather than the creature"
    );
}
