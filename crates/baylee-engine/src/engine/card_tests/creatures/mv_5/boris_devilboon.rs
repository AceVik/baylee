//! `cards/creatures/mv_5/boris_devilboon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Boris Devilboon — {3}{B}{R} — prints one line: "{2}{B}{R}, {T}: Create a
/// 1/1 black and red Demon creature token named Minor Demon."
///
/// Both halves of that price are the engine's answer rather than the card's, so
/// both are played on one board: ten lands pay the five the cast costs, and a
/// later turn pays the four the ability charges plus the tap symbol the Wizard
/// supplies himself — the `{T}` being why the walk exists, since CR 302.6 keeps
/// a creature that arrived this turn from paying a tap price, and the assertion
/// is made with the ability's mana already floating so sickness is the only
/// thing refusing it. The Demon is read only after the stack has emptied,
/// because making a token is no mana ability: its body, its black-and-red
/// identity and the name the card gives it are three claims a token merely
/// arriving would not tell apart.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn boris_devilboon_taps_and_four_mana_for_a_minor_demon() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[boris_devilboon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The card arrives the way the card arrives: {3}{B}{R} out of the ten
    // lands, and the five that are left over are exactly the pool the
    // ability's {2}{B}{R} is charged against below. A pool empties when a step
    // ends (CR 500.5), which is why each turn here is read in its own main
    // phase.
    cast_from_hand(&mut engine, p0, boris_devilboon());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let boris = on_battlefield(&engine, p0, boris_devilboon()).expect("Boris resolved");
    assert_eq!(pt(&engine, boris), (2, 2), "the body the card prints");
    assert!(!is_tapped(&engine, boris), "and he enters untapped");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    // How the payer split the generic half does not matter: five black and
    // five red went in and one of each is spoken for by the coloured symbols,
    // so what is left can pay {2}{B}{R} however the three generic were spent.
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        5,
        "ten lands tapped less the {{3}}{{B}}{{R}} the cast spent"
    );
    assert!(
        pool.available(ManaColor::Black) >= 1 && pool.available(ManaColor::Red) >= 1,
        "the four the ability charges are payable out of this very pool"
    );

    // ...and the ability is still not offered, which leaves the `{T}` as the
    // only unpaid part of the price: summoning sickness (CR 302.6).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == boris),
        "{{T}} on a creature that arrived this turn cannot be paid, so the line \
         is absent from the offer: {:?}",
        legal.abilities
    );

    // A turn round the table puts him past it and stands the ten lands back up.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(!is_tapped(&engine, boris), "the untap step stood him up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool the cast left emptied with the step that ended (CR 500.5)"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the *pool* and not the untapped lands: with nothing floating the
    // {2}{B}{R} is unpayable and the line is not there at all — the half a test
    // that only ever tapped first would never see.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == boris),
        "{{2}}{{B}}{{R}} is not four, so the cost is unpayable and nothing is \
         offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        10,
        "five Swamps and five Mountains, every source on the board"
    );
    assert!(
        !is_tapped(&engine, boris),
        "and the Wizard stands: his price is not his own {{T}} alone, so \
         `tap_all_mana` may not press it"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(boris, 0)),
        "the one line the card prints, now that its {{2}}{{B}}{{R}} is in the \
         pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, boris_devilboon(), 0);
    assert!(
        is_tapped(&engine, boris),
        "{{T}} is half the price, and CR 601.2h pays it with the rest"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "and the other half was four of the ten mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Demon arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Minor Demon");
    let demon = engine
        .state()
        .object(tokens[0])
        .expect("the Demon is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(demon.name, "Minor Demon", "the name the card gives it");
    assert_eq!(
        (demon.power, demon.toughness),
        (Some(1), Some(1)),
        "the 1/1 body the card prints"
    );
    assert!(
        demon.colors.contains(baylee_core::color::Color::Black)
            && demon.colors.contains(baylee_core::color::Color::Red),
        "black and red, which is what the colour line says and no other colour"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "and it is a creature token, not an artifact or an enchantment"
    );
    assert!(
        on_battlefield(&engine, p0, boris_devilboon()).is_some(),
        "the price was the tap and the mana, so the Wizard is still standing to \
         make another one"
    );
}
