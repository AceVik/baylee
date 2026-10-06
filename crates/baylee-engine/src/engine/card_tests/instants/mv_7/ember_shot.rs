//! `cards/instants/mv_7/ember_shot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ember Shot prints two sentences — "Ember Shot deals 3 damage to any
/// target" and "Draw a card" — and one cast has to show both, so the seat
/// across the table is aimed at rather than the creature on it: a life total
/// that lands exactly on seventeen is the only reading that tells the printed
/// three from a one or a two, while the Llanowar Elves standing on that board
/// is what makes "any target" (CR 115.4) a claim about both option lists
/// instead of about having nothing to point at. Seven Mountains pay the
/// {6}{R} down to an empty pool, so the mana that is gone afterwards is the
/// printed cost really paid rather than a card that resolved for free, and
/// reading the draw as a library one shorter *and* a Shot in the graveyard is
/// what separates a drawn card from a hand that merely changed size.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn ember_shot_deals_three_to_any_target_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[ember_shot()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let shot = in_hand(&engine, p0, ember_shot()).expect("the Shot is in hand");

    // `LegalActions::castable` is filtered through `can_afford`, which reads
    // the *pool* and not the untapped lands — so the claim is made twice, once
    // on an empty pool and once with the mana really floating. The Elf is the
    // control for the other reason a cast is withheld: the spell has a legal
    // target on this board either way.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&shot),
        "an empty pool pays no {{6}}{{R}}, and `can_afford` reads the pool \
         rather than the seven untapped Mountains: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Mountains tapped, seven red, and no creature of mine makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&shot),
        "the seven floating pay the whole cost, so the Shot is castable: {:?}",
        legal.castable
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let my_life = engine.state().players[0].life;
    let their_life = engine.state().players[1].life;

    cast_with_floating(&mut engine, p0, ember_shot());
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
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // mana is still floating while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the cost is the last step of the cast, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{6}}{{R}} came out of the seven the Mountains made"
    );
    assert!(
        !stack_is_empty(&engine),
        "an instant resolves off the stack, so it is waiting there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        their_life - 3,
        "\"deals 3 damage to any target\" — three, and not one per mana spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        my_life,
        "the damage went to the target that was named, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and it still carries the body it was printed with"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        in_graveyard(&engine, p0, ember_shot()).is_some(),
        "the resolved instant went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Shot left the hand and the draw put exactly one card back"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&shot),
        "and the card in hand is the drawn one, not the Shot that was cast"
    );
}
