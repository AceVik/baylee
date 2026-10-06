//! `cards/artifacts/mv_2/agatha_s_soul_cauldron.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Agatha's Soul Cauldron prints three abilities and the file says two of
/// them have no spelling, so the one that is written is the whole of what a
/// board can hold it to: "{T}: Exile target card from a graveyard."
///
/// "A graveyard" is the word worth playing, because it is the easy one to
/// narrow by accident — the card says any graveyard and the DSL says so with
/// `PlayerRel::EachPlayer`, so both seats are given a card to lose and the
/// offer has to name both. The card also has to actually *leave*: a target
/// that is still in the graveyard afterwards is an exile that resolved
/// against nothing, and the graveyard count alone would not say which of the
/// two seats paid for it.
#[test]
fn agathas_soul_cauldron_exiles_a_card_out_of_either_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[agatha_s_soul_cauldron()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    let mine = *engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .first()
        .expect("p0's graveyard was seeded");
    let theirs = *engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .first()
        .expect("p1's graveyard was seeded");
    let exiled_before = engine.state().zones.list(ZoneLocation::Exile(p1)).len();

    // `Cost::TAP` and nothing else — no mana is on the board at all, which
    // is what says the price is the tap symbol the card prints.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the board has no land on it: the ability costs its own tap and no mana"
    );
    activate(&mut engine, p0, agatha_s_soul_cauldron(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target card from a graveyard\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"a graveyard\" is every graveyard, and one of them is the \
         activating player's own: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the opponent's card was offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .contains(&theirs),
        "the targeted card left the graveyard it was in"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        exiled_before + 1,
        "and it is in exile — under its owner, which is where a card goes \
         and not under the artifact's controller"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&mine),
        "the card the ability did not name is untouched: one target, one exile"
    );
    assert!(
        is_tapped(
            &engine,
            on_battlefield(&engine, p0, agatha_s_soul_cauldron()).expect("still out")
        ),
        "{{T}} was the cost"
    );
}
