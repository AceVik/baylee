//! `cards/instants/mv_4/inspiration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Inspiration — {3}{U} Instant: "Target player draws two cards."
///
/// "Target player" is the whole card, so the target named here is the seat
/// that did *not* cast it: p1's hand grows by two and p1's library shrinks by
/// two, while p0 — who paid the {3}{U} — draws nothing and only loses the card
/// itself to its own graveyard. A printing that read "you draw two" would
/// satisfy every count taken on p0's side of the table and fail both of the
/// ones taken here, and a printing that read "target creature's controller"
/// would have no creature on this board to name. The four Islands are tapped
/// before the cast, because the offer reads the pool and not the untapped
/// lands, and they are the only mana on the board.
#[test]
fn inspiration_draws_two_cards_for_the_player_it_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[inspiration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_library = library_size(&engine, p0);
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let their_library = library_size(&engine, p1);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, inspiration());

    // CR 601.2c before CR 601.2h: the cost is the last step of the cast, so
    // the four blue are still floating while the target question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands tapped for four blue, and the {{3}}{{U}} is not paid yet"
    );

    // A player-only target is a target question whose menu is the seats of
    // the game. Whichever of the two forms the engine publishes it under, the
    // answer is taken out of that question's own enumeration.
    let offered = match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the seat that cast it names the target");
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the other seat was one of the options");
            player_options
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the seat that cast it names the target");
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the other seat was one of the options");
            options
        }
        other => panic!("expected a target choice, got {other:?}"),
    };
    assert!(
        offered.contains(&p0) && offered.contains(&p1),
        "\"target player\" is either seat of the game: {offered:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 2,
        "\"target player draws two cards\": two off the targeted seat's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand + 2,
        "and both are in that seat's hand, so an emptied library would not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "the caster draws nothing — the cards belong to the player it named"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand - 1,
        "p0's hand is one smaller: the Inspiration itself left it and no draw replaced it"
    );
    assert!(
        in_graveyard(&engine, p0, inspiration()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}}{{U}} was paid rather than merely printed"
    );
}
