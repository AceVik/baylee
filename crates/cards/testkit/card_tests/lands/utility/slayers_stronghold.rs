//! `cards/lands/utility/slayers_stronghold.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Slayers' Stronghold prints two abilities: "{T}: Add {C}" and "{R}{W}, {T}:
/// Target creature gets +2/+0 and gains vigilance and haste until end of
/// turn." Both are played in one game — a Mountain and a Plains pay the
/// coloured half while the land itself is kept back so its own `{T}` can be
/// the other half of the price (`tap_all_mana` would have spent the very
/// permanent under test, #159/#17). The target question is read before
/// anything is paid (CR 601.2c, then 601.2h), and it is a bare
/// `Filter::CREATURE`: the opponent's Elf is on it, so "target creature" is
/// not "target creature you control". The turn is then walked out, because
/// "until end of turn" is part of the card — and the land untapping is what
/// lets the colourless line be read on the same board.
#[test]
#[allow(clippy::too_many_lines)] // one land, both printed lines, and the duration walked out
fn slayers_stronghold_pays_red_white_and_its_own_tap_for_a_two_zero_and_two_keywords() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[slayers_stronghold(), mountain(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let keep = on_battlefield(&engine, p0, slayers_stronghold()).expect("the land is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert!(!is_tapped(&engine, keep), "it entered untapped");

    // The land's own `{T}` is part of the coloured ability's price, and both
    // of its printed lines want that same symbol — so it is named as the one
    // source kept back rather than left to `tap_all_mana` (#159/#17).
    tap_all_mana_but(&mut engine, p0, Some(slayers_stronghold()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain's red"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the Plains' white: both halves of {{R}}{{W}} are floating"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the Elf's own {{G}} beside them — a mana creature is a mana route \
         too, and the helper takes basic-land-type and printed {{T}}: Add … \
         alike (#159)"
    );
    assert!(
        !is_tapped(&engine, keep),
        "the land kept back is still standing, so its {{T}} is there to pay"
    );

    // Ability 0 is the printed `{T}: Add {C}`; ability 1 is the pump.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(keep, 1)),
        "with {{R}}{{W}} in the pool the coloured line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, slayers_stronghold(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
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
        player_options.is_empty(),
        "a creature is no seat: \"any target\" would have carried players too"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&keep),
        "the land is no creature: {options:?}"
    );

    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, keep),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{R}}{{W}} is still floating for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("my Elf was one of the options the question enumerated");

    assert!(is_tapped(&engine, keep), "{{T}} is paid by the land itself");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "both halves of {{R}}{{W}} came out of the pool, and the Elf's {{G}} \
         is what is left"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "the red is spent"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "and the white beside it"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, mine),
        (3, 1),
        "+2/+0 on the creature the ability named"
    );
    let granted = keywords(&engine, mine);
    assert!(granted.contains(KeywordSet::VIGILANCE), "and vigilance");
    assert!(granted.contains(KeywordSet::HASTE), "and haste");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "nor do the keywords"
    );

    // "until end of turn": one whole turn later the Elf is a printed 1/1
    // again, and the untap step has stood the land back up for its other line.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "and it took the keywords with it"
    );
    assert!(
        !is_tapped(&engine, keep),
        "the untap step stood the land up"
    );

    // The second printed line, on the turn its `{T}` is payable again.
    activate(&mut engine, p0, slayers_stronghold(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — the other half of the card"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, keep), "and the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
}
