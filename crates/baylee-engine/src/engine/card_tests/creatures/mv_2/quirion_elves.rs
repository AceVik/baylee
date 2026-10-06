//! `cards/creatures/mv_2/quirion_elves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Quirion Elves — {1}{G}, 1/1 Elf Druid — prints three sentences: "As this
/// creature enters, choose a color", "{T}: Add {G}", and "{T}: Add one mana of
/// the chosen color". Two mana lines on one body that differ only in the color
/// they hand back, so both are played: the {G} on the turn the Elf can finally
/// pay its own {T} (CR 302.6 keeps a fresh 1/1 from tapping the turn it
/// arrives), and the chosen color on the next one — and the second tap asks
/// nothing, because the color was settled once as the creature entered. Each
/// activation is read against an empty pool, so "exactly one green" and "exactly
/// one blue" are claims about the Elf rather than about the Forests it was cast
/// with. The color is checked through the pool and not through a field, because
/// the pool is what the card is for.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn quirion_elves_taps_for_green_and_for_the_color_it_chose_as_it_entered() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[quirion_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{G} off the two Forests, which spends both: the pool is empty again
    // by the time the Elf asks its question.
    cast_from_hand(&mut engine, p0, quirion_elves());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} took the two Forests and no source is left floating"
    );

    for _ in 0..20 {
        if matches!(engine.pending(), Pending::ChooseColor { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "expected priority while the Elves resolves, got {:?}",
                engine.pending()
            );
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"as this creature enters, choose a color\" is a question, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        player, p0,
        "the controller of the entering creature chooses"
    );
    assert_eq!(
        options.len(),
        5,
        "a color is one of the five, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue),
        "blue is one of the colors it offered: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colors it offered");

    let elves = on_battlefield(&engine, p0, quirion_elves()).expect("the Elves resolved");
    assert!(
        stack_is_empty(&engine),
        "the question was the spell's last step, so nothing is waiting behind it"
    );
    assert_eq!(pt(&engine, elves), (1, 1), "the printed body");

    // A creature pays a {T} only from the turn after it arrived (CR 302.6),
    // so the first tap is on p0's next turn and not in the main phase it was
    // cast in. Nothing has been tapped and nothing floats.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, elves),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and no mana is left over from the cast"
    );

    // Ability 0 is the printed "{T}: Add {G}".
    activate(&mut engine, p0, quirion_elves(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "{{T}}: Add {{G}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here already"
    );
    assert!(is_tapped(&engine, elves), "the Elf paid its own {{T}}");

    // A whole turn later, for the same reason: one creature has one {T}.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool emptied when the step that held the green ended (CR 500.5)"
    );
    assert!(!is_tapped(&engine, elves), "and the Elf untapped again");

    // Ability 1 is the printed "{T}: Add one mana of the chosen color", and
    // the color was chosen as the creature entered: nothing is asked here.
    activate(&mut engine, p0, quirion_elves(), 1);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the color was settled once, on entry, so the tap asks nothing: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "\"one mana of the chosen color\", and blue is what was chosen"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the second line names no color of its own, so it adds none of the first's"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and nothing beside it"
    );
    assert!(
        is_tapped(&engine, elves),
        "the Elf paid its own {{T}} again"
    );
}
