//! `cards/lands/manlands/hissing_quagmire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hissing Quagmire is a creature land: it enters tapped, taps for {B} or
/// {G}, and for `{1}{B}{G}` becomes a 2/2 black and green Elemental with
/// deathtouch until end of turn — a price with no tap symbol in it.
///
/// That missing `{T}` is why the animation is pressed on a land that is
/// already sideways: the mana ability tapped it a moment before, and the
/// ability still has to be offered, which no reading of the card file can
/// show. The turn cycle is what buys the mana half at all (CR 502.3): a land
/// that entered tapped adds nothing in the turn it arrives, so its own `{T}`
/// is payable only at its controller's next main phase.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn hissing_quagmire_taps_for_black_or_green_and_animates_while_already_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), swamp()])
        .hand(0, &[hissing_quagmire()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The printed entry line has to be *played*, not seeded:
    // `SeatSpec::starting_battlefield` places a permanent with `Cause::Setup`,
    // and a placement is not an entry, so the tapped clause would never run.
    let quagmire = play_land(&mut engine, p0, hissing_quagmire());
    assert!(entered_tapped(&engine, quagmire), "it enters tapped");
    let field = on_battlefield(&engine, p0, forest()).expect("a Forest stands beside it");
    assert!(
        !is_tapped(&engine, field),
        "while the lands the starting battlefield placed are untapped"
    );

    // A turn further, which is where a land that entered tapped can pay for
    // an ability of its own.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, quagmire),
        "the untap step stood the Quagmire back up"
    );

    // Ability 0, "{T}: Add {B} or {G}": two printable colours are a question.
    activate(&mut engine, p0, hissing_quagmire(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "'{{B}} or {{G}}' is a colour choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(options.len(), 2, "two colours, and no third: {options:?}");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "the two the card prints: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the colour that was named, off one tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one land tapped, one mana in the pool"
    );
    assert!(is_tapped(&engine, quagmire), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );

    // Ability 1, "{1}{B}{G}: Until end of turn, this land becomes a 2/2 black
    // and green Elemental creature with deathtouch. It's still a land."
    // The three mana come off the lands beside it, and the ability is still
    // offered there, because nothing in its price says {T}.
    tap_all_mana_but(&mut engine, p0, Some(hissing_quagmire()));
    let before = engine.state().players[0].mana_pool.total();
    assert_eq!(
        before, 4,
        "two Forests and a Swamp are three more, on top of the one the \
         Quagmire just made"
    );

    activate(&mut engine, p0, hissing_quagmire(), 1);
    assert!(
        is_tapped(&engine, quagmire),
        "a land already sideways still animates: the cost is mana and no tap"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the animation is no mana ability, so it waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        before - 3,
        "{{1}}{{B}}{{G}} came out of the pool"
    );
    let chars = engine
        .state()
        .object(quagmire)
        .expect("an animated land is still on the battlefield")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::CREATURE) && chars.types.contains(TypeSet::LAND),
        "it is a creature and it is still a land: {:?}",
        chars.types
    );
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(2), Some(2)),
        "the body the card prints"
    );
    assert!(
        chars.keywords.contains(KeywordSet::DEATHTOUCH),
        "and the deathtouch printed with it"
    );

    // "Until end of turn" is not decoration: one turn boundary later the
    // permanent is a plain land again.
    reach_their_main_phase(&mut engine, p1);
    let after = engine
        .state()
        .object(quagmire)
        .expect("the land is still there")
        .characteristics();
    assert!(
        after.types.contains(TypeSet::LAND) && !after.types.contains(TypeSet::CREATURE),
        "the animation lasted the turn it was paid for and no longer: {:?}",
        after.types
    );
}
