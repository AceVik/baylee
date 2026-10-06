//! `cards/creatures/mv_4/wirewood_channeler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wirewood Channeler — {3}{G} Creature — Elf Druid, 2/2, printing one line:
/// "{T}: Add X mana of any one color, where X is the number of Elves on the
/// battlefield."
///
/// Every word of that sentence is read somewhere on this board. Four Elves
/// stand on it — the Channeler itself, two Llanowar Elves under its
/// controller and one across the table — beside a Baleful Strix, which is a
/// creature and no Elf, so the count of four rules out both "Elves you
/// control" (three) and "creatures on the battlefield" (five) and shows that
/// an opponent's Elf is counted. The colour is the printed "any one *color*":
/// five options, and the four black that land afterwards have no other source
/// on a board where nothing was ever tapped.
#[test]
fn wirewood_channeler_makes_one_mana_per_elf_on_the_battlefield_in_one_named_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                wirewood_channeler(),
                llanowar_elves(),
                llanowar_elves(),
                baleful_strix(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let channeler =
        on_battlefield(&engine, p0, wirewood_channeler()).expect("the Channeler is out");
    assert_eq!(pt(&engine, channeler), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so every mana read below came off the Channeler"
    );

    // `{T}` and not a drop of mana is the whole price, so an empty pool
    // withholds nothing: a mana ability a card *prints* has an index to name
    // and is an ordinary entry in `abilities`, never the CR 305.6 shortcut.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(channeler, 0)),
        "an untapped Channeler is a paid {{T}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, wirewood_channeler(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"any one color\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
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

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        is_tapped(&engine, channeler),
        "the Channeler paid its own {{T}}"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        4,
        "one mana per Elf on the battlefield — the Channeler, my two Llanowar \
         Elves and the one across the table — while the Baleful Strix is a \
         creature and no Elf"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\"any one color\" is one color and not one of each, and no Llanowar \
         Elves were tapped to fill the pool"
    );
    assert_eq!(pool.total(), 4, "four black, and nothing beside them");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
