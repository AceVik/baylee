//! `cards/creatures/mv_4/phyrexian_defiler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Defiler prints one line: "{T}, Sacrifice this creature: Target
/// creature gets -3/-3 until end of turn."
///
/// Both halves of that price are the engine's answer rather than the card's, so
/// both are read inside one activation: while the target question stands
/// (CR 601.2c) the Defiler is still an untapped 3/3 on the battlefield, and the
/// answer to it (CR 601.2h) spends the tap and the creature at once, leaving
/// the card in its owner's graveyard before the ability has even resolved. No
/// part of that price is mana, so the empty pool is asserted rather than filled.
///
/// The target is the 4/4 across the table rather than a 1/1: a body that
/// survives is the only one that can read both printed numbers, where a 1/1
/// would die and prove the toughness half alone. The Elf beside the Defiler is
/// the filter's control — "target creature" reaches one creature and not the
/// board — and a whole turn later the same 4/4 is back, which is what says the
/// -3/-3 is the duration the card prints and not a body the game keeps.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn phyrexian_defiler_sacrifices_itself_to_give_a_creature_minus_three_minus_three() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[phyrexian_defiler(), quiet_creature()])
        .battlefield(1, &[air_elemental()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let defiler = on_battlefield(&engine, p0, phyrexian_defiler()).expect("the Defiler is out");
    let control = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let victim = on_battlefield(&engine, p1, air_elemental()).expect("their Elemental is out");
    assert_eq!(pt(&engine, defiler), (3, 3), "the body the card prints");
    assert_eq!(
        pt(&engine, victim),
        (4, 4),
        "a 4/4 for -3/-3 to leave alive"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap symbol and the creature, and neither is mana"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that reads
    // the pool rather than the untapped lands — an empty pool withholds nothing
    // here, because no part of this price is mana.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(defiler, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, phyrexian_defiler(), 0);
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
        options.contains(&control) && options.contains(&victim),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // Defiler is still standing while this question is open.
    assert!(
        on_battlefield(&engine, p0, phyrexian_defiler()).is_some(),
        "the price is the last step of the activation, not the first"
    );
    assert!(
        !is_tapped(&engine, defiler),
        "and nothing has tapped it yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elemental across the table was one of the options");

    assert!(
        on_battlefield(&engine, p0, phyrexian_defiler()).is_none(),
        "the sacrifice is part of the price, so the Defiler left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, phyrexian_defiler()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "a -3/-3 is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        pt(&engine, victim),
        (4, 4),
        "and nothing has happened to the target yet: the pump is the resolution, \
         not the cost"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "-3/-3 on the 4/4 the ability named: a body that survives is what makes \
         both halves of the printed number readable"
    );
    assert_eq!(
        pt(&engine, control),
        (1, 1),
        "the Elf beside it is untouched — the effect targets, it does not sweep \
         the board"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the whole price was the tap and the creature: no mana was ever in \
         the pool"
    );

    // "until end of turn": a turn later the Elemental is the 4/4 it was, so the
    // -3/-3 was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, victim),
        (4, 4),
        "the shrink lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p1, air_elemental()).is_some(),
        "and the creature is still standing, so the pump left rather than the \
         creature"
    );
}
