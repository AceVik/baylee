//! `cards/enchantments/mv_3/armistice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Armistice is a `{2}{W}` enchantment with exactly one line: "{3}{W}{W}:
/// You draw a card and target opponent gains 3 life." The scenario plays
/// both halves in one main-phase window, because eight Plains first pay the
/// enchantment spell and afterwards leave exactly the five mana that the
/// ability demands (CR 500.5) — so the cost is a real payment and not a
/// label on a free ability. The table with three seats is the word the card
/// relies on: the offer names both other seats and never its own, and the
/// three life lands on the one named, while the third seat stays untouched;
/// the draw is the half that no pool read can see.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn armistice_draws_a_card_and_gives_one_named_opponent_three_life() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, plains(), 3)
        .battlefield(0, &[plains(); 8])
        .hand(0, &[armistice()])
        .life(0, 20)
        .life(1, 20)
        .life(2, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Mana first: `cast_from_hand` taps the board, so the pool is read and
    // not the untapped lands, and what is left of the eight Plains after the
    // {2}{W} is exactly the {3}{W}{W} the ability goes on to charge.
    cast_from_hand(&mut engine, p0, armistice());
    pass_until(&mut engine, stack_is_empty);
    let enchantment = on_battlefield(&engine, p0, armistice()).expect("the Armistice resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "eight Plains pay the {{2}}{{W}} and leave exactly five for the ability"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "the one line the card prints is offered with its five mana floating: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, armistice(), 0);

    // CR 601.2c before CR 601.2h: while the target question stands no mana
    // has been spent yet, and the answer is lifted out of what the question
    // itself enumerated rather than guessed at.
    let action = match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat aims it");
            assert!(
                options.is_empty(),
                "nothing on the battlefield is a legal target for it: {options:?}"
            );
            assert_eq!(
                player_options.len(),
                2,
                "\"target opponent\" is the two other seats: {player_options:?}"
            );
            assert!(
                player_options.contains(&p1) && player_options.contains(&p2),
                "both opponents are offered: {player_options:?}"
            );
            assert!(
                !player_options.contains(&p0),
                "\"target opponent\" is not \"target player\": {player_options:?}"
            );
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![p1],
            }
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the activating seat aims it");
            assert_eq!(options.len(), 2, "the two other seats: {options:?}");
            assert!(
                options.contains(&p1) && options.contains(&p2) && !options.contains(&p0),
                "both opponents and never the controller: {options:?}"
            );
            PlayerAction::ChoosePlayer(p1)
        }
        other => panic!("\"target opponent\" is a target choice, got {other:?}"),
    };
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the cost is the last step of the activation, so the mana is still floating"
    );
    engine
        .apply(p0, action)
        .expect("the seat the question offered is a legal answer");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{W}}{{W}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "and it is no mana ability");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        23,
        "\"target opponent gains 3 life\" — the seat that was named"
    );
    assert_eq!(
        engine.state().players[2].life,
        20,
        "and exactly one opponent: the second seat never moved"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the life belongs to the opponent, not to the seat that paid"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "\"You draw a card\""
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library, so the hand grew by a draw"
    );
    assert!(
        on_battlefield(&engine, p0, armistice()).is_some(),
        "an activated ability costs the enchantment nothing"
    );
}
