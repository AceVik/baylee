//! `cards/artifacts/mv_1/sol_ring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Conduit of Worlds: "{T}: Choose target nonland permanent card in your
/// graveyard. If you haven't cast a spell this turn, you may cast that card.
/// If you do, you can't cast additional spells this turn."
///
/// One Forest is tapped first, and Sol Ring is offered on that floating
/// green. No spell has been cast this turn, so the resolving ability asks;
/// a yes opens a payment window for the Elves' {G}, the seat taps its other
/// Forests, and passing casts the Elves off the graveyard, paid out of the
/// pool. The Elves resolve onto the battlefield. Sol Ring, with two green
/// still floating, is then not offered: the lock is on.
#[test]
fn conduit_of_worlds_casts_a_graveyard_card_and_locks_further_spells() {
    let p0 = PlayerId::new(0);
    let (mut engine, elves) = a_conduit_with_elves_in_the_graveyard();
    let ring = in_hand(&engine, p0, sol_ring()).expect("the Ring is in hand");
    let first_forest = on_battlefield(&engine, p0, forest()).expect("a Forest stands");
    assert_eq!(tap_mana_where(&mut engine, p0, |id| id == first_forest), 1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ring),
        "the Ring is castable before the Conduit is used"
    );

    conduit_targets(&mut engine, elves);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0);
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::CastPaying { card: elves },
        "\"you may cast that card\""
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            p0,
            baylee_core::mana::ManaPayment::Fixed(baylee_core::mana::ManaCost::parse("{G}"))
        )),
        "the window owes the card's mana cost"
    );
    assert_eq!(tap_all_mana(&mut engine, p0), 2, "the other two Forests");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(engine.payment_window().is_none());
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the Elves were cast from the graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "paid {{G}} out of the three made"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(
        !legal.castable.contains(&ring),
        "\"you can't cast additional spells this turn\""
    );
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: ring })
            .is_err(),
        "and a cast named anyway is refused"
    );
}

/// Conduit of Worlds: "If you haven't cast a spell this turn". The seat
/// casts Sol Ring first; the Conduit's ability then resolves without asking,
/// and the Elves stay in the graveyard.
#[test]
fn conduit_of_worlds_offers_nothing_once_a_spell_was_cast_this_turn() {
    let p0 = PlayerId::new(0);
    let (mut engine, elves) = a_conduit_with_elves_in_the_graveyard();
    cast_from_hand(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, sol_ring()).is_some());

    conduit_targets(&mut engine, elves);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "nothing is offered"
    );
    assert!(engine.payment_window().is_none());
    assert_eq!(
        engine.state().object(elves).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the Elves stay where they are"
    );
}

/// Jade Monolith's activated ability is targeted, naming `target creature`
/// (CR 115.1c: "target [something]" identifies what the ability affects,
/// described by the phrase after "target"): the offer lists creatures
/// only, whoever controls them, and no player at all — a noncreature
/// artifact beside them is left off the menu.
#[test]
fn jade_monoliths_activated_ability_offers_creatures_only_as_its_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(
            1,
            &[jade_monolith(), llanowar_elves(), sol_ring(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("seated");
    let monolith = on_battlefield(&engine, p1, jade_monolith()).expect("seated");

    reach_their_main_phase(&mut engine, p1);
    let land = on_battlefield(&engine, p1, forest()).expect("their land");
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays for the {1}");
    activate(&mut engine, p1, jade_monolith(), 0);
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
            "expected the Monolith's own target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((player, min, max), (p1, 1, 1), "one target creature");
    assert!(
        options.contains(&giant) && options.contains(&elf),
        "any creature, whoever controls it: {options:?}"
    );
    assert!(
        !options.contains(&ring) && !options.contains(&monolith),
        "a noncreature artifact is not \"target creature\": {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "\"target creature\", never a player: {player_options:?}"
    );
}
