//! `cards/lands/deserts/jagged_barrens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jagged Barrens prints three lines: it enters tapped, its arrival deals 1
/// damage to target opponent, and it taps for {B} or {R}. The table is three
/// seats so that "target opponent" is a real choice — the question has to
/// offer both other seats and never the land's own controller — and the
/// scenario crosses a full turn cycle, because a land that arrived tapped has
/// no {T} to spend until its controller's next untap step. That crossing is
/// what makes the colour question at the end evidence of the printed mana
/// ability rather than of a land that had been standing all along.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn jagged_barrens_enters_tapped_pings_a_chosen_opponent_and_taps_for_black_or_red() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .hand(0, &[jagged_barrens()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main"
    );

    let land = in_hand(&engine, p0, jagged_barrens()).expect("the Barrens are in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .expect("an empty first main phase has nothing in the way of a land drop");

    // The enters trigger (CR 603.6a) targets as it is put on the stack, so
    // the question arrives with the land already on the battlefield.
    let mut offered: Vec<PlayerId> = Vec::new();
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "the land's controller chooses");
                offered = options.clone();
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .expect("the question's own list holds the answer");
            }
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "the land's controller chooses");
                offered = player_options.clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("an opponent is a legal target for \"target opponent\"");
            }
            Pending::Priority { player, .. } => {
                if stack_is_empty(&engine) && !offered.is_empty() {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Barrens' trigger resolves: {other:?}"),
        }
    }
    assert!(stack_is_empty(&engine), "the trigger has resolved");
    assert_eq!(
        offered.len(),
        2,
        "\"target opponent\" is every other seat: {offered:?}"
    );
    assert!(
        offered.contains(&p1) && offered.contains(&p2),
        "both opponents are on the menu: {offered:?}"
    );
    assert!(
        !offered.contains(&p0),
        "and the land's own controller is not: {offered:?}"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the seat that was named took the point"
    );
    assert_eq!(
        engine.state().players[2].life,
        20,
        "the seat that was not named did not"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"target *opponent*\" never points at the controller"
    );

    // "This land enters tapped", read as an *offer*: a {T} the permanent
    // cannot pay is not in `legal.abilities`, and the pool is empty, so
    // nothing else could have stood in for it.
    assert!(is_tapped(&engine, land), "the Barrens arrived tapped");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "p0 holds priority in their own main phase, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a land that entered tapped has no {{T}} to spend this turn: {:?}",
        legal.abilities
    );

    // A turn cycle later the same permanent is offered — index 0 is the
    // enters trigger, index 1 the printed mana ability — and the line it
    // offers is the printed choice rather than "any color".
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p2);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the Barrens back up"
    );

    activate(&mut engine, p0, jagged_barrens(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a colour question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the two colours the land is printed for, and no third"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two the land offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named"
    );
    assert_eq!(pool.available(ManaColor::Black), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "and the land is what paid for it");
}
