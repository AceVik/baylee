//! `cards/lands/deserts/forlorn_flats.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forlorn Flats is a Desert that enters tapped, deals 1 damage to a chosen
/// opponent as it arrives, and taps for {W} or {B}.
///
/// One game reads all three sentences. The damage is the half a board cannot
/// fake: p0's own twenty is the control, because "target **opponent**" is the
/// word the trigger turns on, and the only life that moves is p1's. The mana
/// line needs the turn cycle — a land that arrives tapped has no `{T}` in the
/// turn it arrives — so p0 is walked around to its next main phase and the
/// two printed colours are shown to be a real choice rather than a default.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn forlorn_flats_enters_tapped_shocks_the_opponent_and_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[forlorn_flats()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, forlorn_flats());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — the printed enter modifier, not a \
         choice anybody was asked for"
    );

    // The enters trigger asks which opponent. A single legal answer may be
    // resolved by the engine without a question, so both arrivals are
    // answered and the assertion below is about the damage rather than about
    // who picked the target.
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&p1) && options.is_empty(),
                    "\"target opponent\" is a player and not a thing, and the \
                     table's one other seat is the whole menu: objects \
                     {options:?}, players {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("the opponent the question offered");
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(options, vec![p1], "one opponent, one answer");
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .expect("the opponent the question offered");
            }
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the enters trigger resolves: {other:?}"),
        }
    }

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"it deals 1 damage to target opponent\" — the opponent, and one"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the land's own controller is not an opponent of itself"
    );

    // Around a turn: the land is still tapped until its controller's next
    // untap step, and its {T} is not even in the offer until then.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so whatever the pool holds afterwards is this land's"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "the printed {{T}} is offered — ability 0 is the enters trigger, \
         which is no activation: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, forlorn_flats(), 1);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "both printed colours are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else is: the card prints two colours, not a third"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours the land offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, with no other source"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
