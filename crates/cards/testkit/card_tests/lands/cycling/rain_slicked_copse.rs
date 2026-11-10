//! `cards/lands/cycling/rain_slicked_copse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rain-Slicked Copse prints three lines and every one of them has to be
/// *played* to be read: it is a Forest Island, "this land enters tapped", and
/// "Cycling {2}". The tapped entry needs a real land drop — `starting_battlefield`
/// places a permanent with no entry at all, so a Copse seated there would stand
/// untapped whatever the card says — while the cycle is the second copy paying
/// {2} and discarding itself out of hand, which no board state can show. The
/// mana line is the one that has to wait: a land that arrives tapped offers
/// nothing until its controller's next untap step, and that step is where
/// "Add {G} or {U}" becomes the question both basic land types make it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn rain_slicked_copse_enters_tapped_cycles_itself_away_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[rain_slicked_copse(), rain_slicked_copse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, and a real one: a Copse put on the board by the harness
    // is a placement rather than an entry, and the assertion below would be
    // reading the harness and not the card.
    let copse = play_land(&mut engine, p0, rain_slicked_copse());
    assert!(
        entered_tapped(&engine, copse),
        "\"This land enters tapped\", and it was played rather than seated"
    );

    // Cycling {2} off the copy still in hand. The two Forests are the price,
    // which doubles as the reading on the tapped Copse: it is a land on the
    // battlefield and the pool is exactly two.
    let library_before = library_size(&engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(rain_slicked_copse()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, and the tapped Copse adds nothing to them"
    );

    let card = in_hand(&engine, p0, rain_slicked_copse()).expect("the second copy is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase holds priority: {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == card)
        .expect("cycling is an ability of a card in hand, and {2} is floating for it");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("{{2}} and the card itself pay for the cycle");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} is a cost, paid as the ability is activated (CR 601.2h)"
    );
    assert!(
        in_graveyard(&engine, p0, rain_slicked_copse()).is_some(),
        "\"Discard this card\" puts the very card that cycled into the graveyard"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\", once the cycle leaves the stack"
    );
    assert!(
        on_battlefield(&engine, p0, rain_slicked_copse()).is_some(),
        "the copy that was played is still on the battlefield: one card cycled"
    );

    // A turn later, the other half of the entry: the land stands up and taps.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, copse),
        "the untap step ran — the Copse was down for the whole turn it arrived"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the next main phase holds priority: {:?}", engine.pending())
    };
    // Which of the two lists carries it is the engine's business: a Forest
    // Island has the CR 305.6 shortcut by virtue of its type line, and the
    // card's own `{T}` is the printed half of the same thing.
    let route = if legal.mana_abilities.contains(&copse) {
        PlayerAction::ActivateManaAbility { source: copse }
    } else {
        let (_, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(source, _)| *source == copse)
            .expect("a Forest Island with an untapped {{T}} prints a mana ability");
        PlayerAction::ActivateAbility {
            source: copse,
            ability_index,
        }
    };
    engine
        .apply(p0, route)
        .expect("an untapped land makes its mana");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "one land with two basic land types: \"Add {{G}} or {{U}}\" is a \
             question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that taps is the one that names it");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both basic land types it is printed with, and neither one alone: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not the other one it offers"
    );
    assert_eq!(
        pool.total(),
        1,
        "one land tapped, one mana made, and no second source moved"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting"
    );
}
