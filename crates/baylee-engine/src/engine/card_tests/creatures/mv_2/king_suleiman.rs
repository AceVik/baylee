//! `cards/creatures/mv_2/king_suleiman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// King Suleiman — {1}{W} 1/1 Human Noble: "{T}: Destroy target Djinn or
/// Efreet."
///
/// No card in the pool is printed with either type as a *card*, so the Djinn is
/// made and not found: a Mutavault animated by its own `{1}` is "a 2/2 creature
/// with all creature types", which is a Djinn and an Efreet at once, and the
/// Llanowar Elves standing beside it is the control that says the filter is not
/// "target creature". The turn between the cast and the `{T}` is load-bearing
/// too: a creature's `{T}` ability is not offered the turn it arrives
/// (CR 302.6), so the Noble has to untap before either claim holds.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn king_suleiman_destroys_the_animated_mutavault_that_is_every_creature_type() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), mutavault(), llanowar_elves()],
        )
        .hand(0, &[king_suleiman()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{W} off the three Plains and the two mana permanents beside them: the
    // card has to arrive in a game before anything is claimed about its text.
    cast_from_hand(&mut engine, p0, king_suleiman());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let suleiman = on_battlefield(&engine, p0, king_suleiman()).expect("the Noble resolved");

    // A whole turn cycle. `reach_their_main_phase` crosses the combat steps
    // and does not answer "you are already there" from the main phase this
    // started in, which is what a `{T}` ability on a summoning-sick creature
    // needs.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, suleiman),
        "the untap step stood the Noble back up, which is what its {{T}} needs"
    );
    let vault = on_battlefield(&engine, p0, mutavault()).expect("the Mutavault is still out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("and so are the Elves");

    // Mana before the claim: the animation's {1} is read off the pool and not
    // off the untapped lands, so everything taps first — the Mutavault with
    // them, since its whole price is its own {T} (#159). Its animation costs
    // {1} and no tap, so it is still activatable while tapped.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, animation) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == vault)
        .expect("with {{1}} floating, the animation is the one line left on it");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: animation,
            },
        )
        .expect("the animation costs {{1}} and nothing else");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        types(&engine, vault).contains(TypeSet::CREATURE),
        "\"Mutavault becomes a 2/2 creature ... It's still a land\""
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(suleiman, 0)),
        "an untapped Noble with a Djinn on the table is offered the only line \
         it prints: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, king_suleiman(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Djinn or Efreet\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&vault),
        "the animated Mutavault has every creature type and is therefore a Djinn \
         and an Efreet: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Llanowar Elves beside it is neither, so this is a filter and not \
         \"target creature\": {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and those two readings are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![vault],
            },
        )
        .expect("the animated land was one of the options it enumerated");
    // CR 601.2c names the target and CR 601.2h pays, so the {T} is read after
    // the answer and not while the question was still open.
    assert!(is_tapped(&engine, suleiman), "{{T}} was the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, mutavault()).is_none(),
        "the Djinn left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, mutavault()).is_some(),
        "destroyed, so the card is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the filter never named is untouched"
    );
    assert!(
        on_battlefield(&engine, p0, king_suleiman()).is_some(),
        "and the Noble is still standing, its ability having aimed elsewhere"
    );
}
