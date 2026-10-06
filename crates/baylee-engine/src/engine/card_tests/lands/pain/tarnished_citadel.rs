//! `cards/lands/pain/tarnished_citadel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tarnished Citadel prints two mana abilities that price themselves in the
/// same `{T}`: "Add {C}", and "Add one mana of any color. This land deals 3
/// damage to you." Because both pay the land's own tap, the two lines can
/// only be told apart across an untap step — so the scenario presses the
/// harmless `{C}` line on the turn the land arrives, walks a full turn cycle
/// (which also shows the pool empties, CR 500.5), and presses the coloured
/// line on the next. The `{C}` tap is the control: same land, same cost, same
/// life total and no damage; the only things that move between the two are
/// which line was activated and the three life the second one costs.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn tarnished_citadel_taps_for_colorless_for_free_and_for_a_color_at_three_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[tarnished_citadel()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land arrives the way a land arrives: played, not seeded, and
    // untapped — nothing on this card makes it enter otherwise, and no
    // creature rule slows a land down (CR 302.6 is a creature rule).
    let land = in_hand(&engine, p0, tarnished_citadel()).expect("the Citadel is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .expect("the land drop is open on turn one");
    let citadel =
        on_battlefield(&engine, p0, tarnished_citadel()).expect("the Citadel is on the table");
    assert!(!is_tapped(&engine, citadel), "it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool is empty before anything is pressed"
    );

    // The {C} line. Its whole price is the land's own {T}, so no colour is
    // asked for and no life is paid.
    activate(&mut engine, p0, tarnished_citadel(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{C}} off the printed line"
    );
    assert_eq!(pool.total(), 1, "and nothing beside it");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the {{C}} line costs no life; only the coloured one does"
    );
    assert!(is_tapped(&engine, citadel), "the {{T}} paid for it");
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack (CR 605.3b)"
    );

    // Both lines pay the same {T}, so only an untap step makes the second
    // reachable at all — and the {C} is gone with the step it was made in.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, citadel), "the untap step ran");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{C}} did not survive into the next turn (CR 500.5)"
    );

    // The coloured line: "any color" is a question (CR 105.4), the answer
    // lands in the pool, and the land's own three damage lands with it. Both
    // questions are taken out of the enumeration the engine itself
    // published, ordered however the ability asks them.
    activate(&mut engine, p0, tarnished_citadel(), 1);
    let mut asked_color = false;
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseColor { player, options } => {
                assert_eq!(player, p0, "the activating seat names the colour");
                assert_eq!(
                    options.len(),
                    5,
                    "the five colours of the game, and colorless is no colour (CR 105.4): {options:?}"
                );
                assert!(
                    options.contains(&ManaColor::Black),
                    "and black is one of them: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChooseColor(ManaColor::Black))
                    .expect("black was one of the colours it offered");
                asked_color = true;
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0);
                assert_eq!(
                    options,
                    vec![p0],
                    "\"deals 3 damage to you\" names the activating seat and nobody else"
                );
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p0))
                    .expect("the seat the damage names is a legal answer");
            }
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert_eq!(player, p0);
                assert_eq!(
                    player_options,
                    vec![p0],
                    "\"deals 3 damage to you\" names the activating seat and nobody else"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p0],
                        },
                    )
                    .expect("the seat the damage names is a legal answer");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the {{T}} line resolves: {other:?}"),
        }
    }
    assert!(
        asked_color,
        "\"one mana of any color\" is a question, and it was never asked"
    );

    assert_eq!(
        engine.state().players[0].life,
        17,
        "\"This land deals 3 damage to you\": the controller pays three"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the opponent pays nothing — the damage is the land's own rider and no burn spell"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana off the one tap");
    assert!(
        is_tapped(&engine, citadel),
        "the second line paid the same {{T}}"
    );
    assert!(
        stack_is_empty(&engine),
        "still a mana ability, so nothing was ever on the stack (CR 605.3b)"
    );
}
