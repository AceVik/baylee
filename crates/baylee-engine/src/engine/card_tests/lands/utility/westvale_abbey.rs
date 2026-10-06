//! `cards/lands/utility/westvale_abbey.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Westvale Abbey's front face is a colourless land with two written lines:
/// "`{T}`: Add `{C}`" and "`{5}`, `{T}`, Pay 1 life: Create a 1/1 white and
/// black Human Cleric creature token." Both are played, a turn apart because
/// they share the one tap symbol, and every part of the second price is read
/// where it lands: the `{5}` out of a pool only the tapped Forests filled (the
/// offer is claimed with the mana already floating, since `can_afford` reads
/// the pool and not the lands), the `{T}` as a status change on the Abbey, and
/// the life as a life total one lower. The Cleric is read only once the stack
/// has emptied — making a token is no mana ability — and both of its printed
/// colours are asserted, because a token that was merely white would still
/// fill the body and the name.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn westvale_abbey_taps_for_colorless_and_sells_a_life_for_a_black_white_cleric() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                westvale_abbey(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let abbey = on_battlefield(&engine, p0, westvale_abbey()).expect("the Abbey is on the table");
    assert!(
        !is_tapped(&engine, abbey),
        "a land placed on the battlefield arrives untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing floats before anything is tapped"
    );

    // Ability 0 is the printed "{T}: Add {C}": a fixed colourless, so nothing
    // is asked on the way and the mana is in the pool the moment it is
    // activated (CR 605.3b). Pressed by hand, because `tap_all_mana` takes
    // this very line (#159) and the tap belongs to the token below.
    activate(&mut engine, p0, westvale_abbey(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is a fixed colourless and not \"any color\", so there is \
         nothing to name: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not a single colour beside it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, abbey), "the Abbey paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting"
    );

    // The tap is spent, so the token line needs the untap step: across the
    // opponent's turn and back, where the pool emptied with the step that
    // ended (CR 500.5) and the Abbey is standing again.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Abbey's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, abbey),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool the {{C}} was in is gone"
    );

    // Six Forests are exactly the {5} the ability charges, and the Abbey is
    // kept back: `tap_all_mana` would have spent the very {T} under test.
    tap_all_mana_but(&mut engine, p0, Some(westvale_abbey()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and an untapped Abbey, which makes no mana of its own"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nobody has paid a life yet"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands — so the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(abbey, 1)),
        "with {{5}} in the pool the token line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, westvale_abbey(), 1);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is part of the price: one, and never a life per mana spent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{5}} came out of the pool, so one green is left"
    );
    assert!(
        is_tapped(&engine, abbey),
        "{{T}} was the other half of the price"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Cleric arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Human Cleric");
    let cleric = engine
        .state()
        .object(tokens[0])
        .expect("the Cleric is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(cleric.name, "Human Cleric", "the name the card gives it");
    assert_eq!(
        (cleric.power, cleric.toughness),
        (Some(1), Some(1)),
        "\"a 1/1\""
    );
    assert!(
        cleric.colors.contains(baylee_core::color::Color::White),
        "\"...white...\""
    );
    assert!(
        cleric.colors.contains(baylee_core::color::Color::Black),
        "and \"...black\": a token that was white alone would fill the body and \
         the name just as well"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "a creature token and not a permanent of some other card type"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid it, not to the opponent"
    );
    assert!(
        on_battlefield(&engine, p0, westvale_abbey()).is_some(),
        "the price was a tap and no sacrifice, so the Abbey stays to make another"
    );
}
