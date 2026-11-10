//! `cards/sorceries/mv_3/symbol_of_unsummoning.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Symbol of Unsummoning is a `{2}{U}` sorcery printing two sentences — "Return
/// target creature to its owner's hand" and "Draw a card" — and one board reads
/// both. The Elf across the table is named, so the bounce has to land it in the
/// hand of the seat that *owns* it rather than the seat that aimed the spell,
/// while the Elf beside the caster is the bystander "target creature" must
/// decline and the cost may not be claimed before the target question is
/// answered (CR 601.2c before CR 601.2h). The draw is read off the library and
/// the hand together, counted from *after* the cast so that the sorcery leaving
/// the hand is not mistaken for the card it drew.
#[test]
fn symbol_of_unsummoning_bounces_the_creature_it_names_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[symbol_of_unsummoning()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Three Islands are exactly {2}{U}, and the caster's own Elf is named as
    // the printing kept back so that "three, three blue" is a claim about the
    // lands and not about a mana creature tapped alongside them.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the three Islands, and the Elf beside them still standing"
    );

    let library_before = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, symbol_of_unsummoning());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the cost is the last step of the cast: nothing has moved yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}} came out of the pool once the target was answered (CR 601.2h)"
    );
    assert!(
        on_stack(&engine, symbol_of_unsummoning()).is_some(),
        "and the sorcery left the hand for the stack, waiting to resolve"
    );

    // The hand is counted after the cast, because while the target question
    // stood the sorcery was still in it (CR 601.2c before CR 601.2h).
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the creature the spell named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it, \
         not to the seat that aimed the spell"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature nobody named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, symbol_of_unsummoning()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the caster's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_cast + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
}
