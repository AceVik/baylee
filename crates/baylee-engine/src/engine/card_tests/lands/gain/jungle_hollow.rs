//! `cards/lands/gain/jungle_hollow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jungle Hollow prints three sentences and each one lands in a different
/// part of the engine: it enters tapped, its entry gains a life, and its `{T}`
/// offers a choice of two colours. The land is *played* rather than seeded
/// because `starting_battlefield` places a permanent without an entry
/// (`Cause::Setup`), so a replacement effect never looks at it and a seeded
/// Hollow would arrive untapped and prove nothing. The two halves are split
/// by a full turn cycle for the same reason on the other side: an
/// entered-tapped land has no `{T}` to pay in the turn it arrives, so nothing
/// about the mana line is readable until its controller's next untap step.
#[test]
fn jungle_hollow_enters_tapped_gains_a_life_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[jungle_hollow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, jungle_hollow());

    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — the printed enter modifier, and the \
         reason it had to be played rather than seeded"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the entry trigger has not resolved yet: it is sitting on the stack"
    );
    pass_until(&mut engine, |e| {
        e.state().players[0].life == life_before + 1
    });
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\" — one life, to the \
         controller, from the entry itself"
    );

    // The tapped entry is not a status flag with nothing behind it: while the
    // land is down, its own `{T}` is not even part of the offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land has no {{T}} to pay: {:?}",
        legal.abilities
    );

    // A whole turn cycle, because an entered-tapped land taps for nothing in
    // the turn it arrives; its controller's next untap step is what stands it
    // back up (CR 502.3).
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, which is what makes the tap below \
         payable at all"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == land)
        .expect("the printed {T} is the only activated ability the land has");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("an untapped land whose whole price is its own tap is offered it");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question with two answers, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert_eq!(
        options.len(),
        2,
        "black and green, and no third colour is printed: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "exactly the two colours the land prints: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "one activation, one mana: the choice is which, never both"
    );
    assert_eq!(pool.total(), 1, "and nothing else is in the pool");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing waits to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
