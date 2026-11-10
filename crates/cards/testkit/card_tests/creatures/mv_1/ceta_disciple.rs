//! `cards/creatures/mv_1/ceta_disciple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ceta Disciple — {U}, a 1/1 Merfolk Wizard printing two activated
/// abilities that each cost a coloured mana *and* its own tap: "{R}, {T}:
/// Target creature gets +2/+0 until end of turn" and "{G}, {T}: Add one mana
/// of any color."
///
/// Because neither whole price is the bare tap symbol, `tap_all_mana` leaves
/// the Disciple standing and taps only the Mountain and the Forest beside it
/// (#159) — which is what lets one board pay for the pump with {R} and, one
/// turn later, pay for the mana ability with {G}; the pool is read before and
/// after each activation so that "the price was paid" is a number.
///
/// The pump's menu is the other half: `Filter::CREATURE` is any creature, so
/// the Elf across the table is offered and the lands are not, while the
/// control is the Disciple itself — it stays a 1/1 while the creature it
/// named becomes a 3/1, and is a 1/1 again the turn after, which is what
/// "until end of turn" means.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn ceta_disciple_pumps_for_red_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[mountain(), forest(), ceta_disciple()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A whole turn cycle first. Nothing here is about summoning sickness, and
    // a creature placed before the game began has to be past it either way —
    // so the two activations below are about the card and not about where the
    // setup happened to put it down.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let disciple = on_battlefield(&engine, p0, ceta_disciple()).expect("the Disciple is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let red_land = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    assert_eq!(pt(&engine, disciple), (1, 1), "a printed 1/1");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and so is the Elf across the table"
    );

    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        2,
        "{{R}} from the Mountain and {{G}} from the Forest"
    );
    assert_eq!(pool.available(ManaColor::Red), 1, "one red");
    assert_eq!(pool.available(ManaColor::Green), 1, "one green");
    assert!(
        !is_tapped(&engine, disciple),
        "both printed abilities cost a mana beside the tap, so neither is a \
         whole price of its own {{T}} and the sweep left the Disciple up"
    );

    // Ability 0: "{R}, {T}: Target creature gets +2/+0 until end of turn."
    activate(&mut engine, p0, ceta_disciple(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert!(
        options.contains(&disciple) && options.contains(&elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&red_land),
        "a land is no creature, and the filter reads the type: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "those two are the whole menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert!(
        is_tapped(&engine, disciple),
        "{{T}} is paid for the ability"
    );
    assert!(!stack_is_empty(&engine), "and the pump is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (3, 1), "+2/+0 on the creature it named");
    assert_eq!(
        pt(&engine, disciple),
        (1, 1),
        "and nothing at all on the creature it did not: the pump is a target \
         and not a board"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0, "the {{R}} was the price");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "while the {{G}} sits untouched"
    );
    assert_eq!(pool.total(), 1, "one of the two mana is left");

    // The Disciple is tapped paying for the pump, so the mana ability is a
    // turn away: its controller's untap step is what stands it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, disciple),
        "the untap step stood the Disciple back up"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and the +2/+0 lasted only the turn it was paid for"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{R}} and {{G}} again, with the Disciple still untapped"
    );

    // Ability 1: "{G}, {T}: Add one mana of any color."
    activate(&mut engine, p0, ceta_disciple(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that activated names it");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the {{G}} was the price"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the Mountain's red was never spent"
    );
    assert_eq!(pool.total(), 2, "one red left over and one blue made");
    assert!(
        is_tapped(&engine, disciple),
        "the Disciple paid its own {{T}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
}
