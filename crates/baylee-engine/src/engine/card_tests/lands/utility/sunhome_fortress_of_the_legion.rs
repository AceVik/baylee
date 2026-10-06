//! `cards/lands/utility/sunhome_fortress_of_the_legion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a9f8344c-1705-4254-81d6-aa05e0c69c29"

/// Sunhome, Fortress of the Legion is a land printing two abilities: "{T}: Add
/// {C}" and "{2}{R}{W}, {T}: Target creature gains double strike until end of
/// turn."
///
/// The activated half is where the card is, and nothing about it can be read
/// off the card file: the four-mana price has to be *in the pool* before the
/// line is even offered (`can_afford` reads the pool, not the untapped lands),
/// and the `{T}` leaves the Fortress down only after the target has been named
/// (CR 601.2c before CR 601.2h). "Target creature" names a board and not a
/// seat, so the Elf across the table sits on the same menu and has to finish
/// without the keyword, and the walk into a later turn is what tells the
/// printed "until end of turn" from a grant the board keeps.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn sunhome_grants_double_strike_to_the_creature_it_names_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                sunhome_fortress_of_the_legion(),
                plains(),
                plains(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let fortress = on_battlefield(&engine, p0, sunhome_fortress_of_the_legion())
        .expect("the Fortress is on the table");
    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "nothing has granted anything yet"
    );

    // Two Plains and two Mountains are exactly `{2}{R}{W}`. The Fortress and
    // the Elf are both named as kept back: the Fortress prints its own
    // "{T}: Add {C}" and `tap_mana_where` would press it (#159), and the Elf
    // is the creature this test reads afterwards.
    tap_mana_where(&mut engine, p0, |id| id != fortress && id != host);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Plains and two Mountains: four mana, and nothing off the Fortress \
         or the Elf"
    );

    // Ability 0 is the printed "{T}: Add {C}"; ability 1 is the pump, and its
    // price is only payable because the mana is already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(fortress, 0)),
        "an untapped Fortress is a paid {{T}}, so its mana line is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(fortress, 1)),
        "and with four mana in the pool the {{2}}{{R}}{{W}} line is offered \
         beside it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sunhome_fortress_of_the_legion(), 1);
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
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&fortress),
        "the Fortress is a land and no creature: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // halves of the price are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, fortress),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and the {{2}}{{R}}{{W}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(
        is_tapped(&engine, fortress),
        "{{T}} is paid by the Fortress itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{R}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "the creature the ability named gained double strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::DOUBLE_STRIKE),
        "the Elf across the table was on the same menu and is still a printed \
         1/1: the effect targets one creature, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, fortress).contains(KeywordSet::DOUBLE_STRIKE),
        "the Fortress grants the keyword, it does not keep it"
    );

    // "until end of turn": a turn later the Elf is a plain 1/1 again, so the
    // keyword was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the keyword left rather than \
         the creature"
    );
}
