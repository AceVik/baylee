//! `cards/instants/mv_6/opportunity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Opportunity is `{4}{U}{U}` for a single printed sentence: "Target player
/// draws four cards." That sentence is `PlayerRel::Chosen`, so the four cards
/// belong to the seat the spell *named* and not to the seat that cast it, and
/// the only way to say so is to name the opponent and read both sides: the
/// targeted library is four shorter and that hand four longer, while the
/// caster's library never moves. The target menu is the other half of the
/// claim — "target player" reaches either chair, so a spell quietly narrowed to
/// its own controller would still resolve and still draw four.
#[test]
fn opportunity_draws_four_for_the_player_it_targets_and_nothing_for_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 6])
        .hand(0, &[opportunity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Islands pay the {4}{U}{U}. Every count is taken before the cast, so
    // that the cards below are read as a move and not as a board.
    let mine = library_size(&engine, p0);
    let theirs = library_size(&engine, p1);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, opportunity());

    // The target is named at CR 601.2c, while the spell is still on the stack.
    // A player-only target is published either as the player half of a target
    // choice or as a bare player choice, so both questions are read.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets { .. } | Pending::ChoosePlayer { .. }
        )
    });
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            min,
            max,
            ..
        } => {
            assert_eq!(player, p0, "the seat that cast the spell names the target");
            assert_eq!((min, max), (1, 1), "one player, and the spell asks once");
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" is any player at the table, the caster \
                 included: {player_options:?}"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the opponent was one of the players the question offered");
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the seat that cast the spell names the target");
            assert!(
                options.contains(&p0) && options.contains(&p1),
                "\"target player\" is any player at the table, the caster \
                 included: {options:?}"
            );
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the opponent was one of the players the question offered");
        }
        other => panic!("Opportunity never asked for a target player: {other:?}"),
    }

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        theirs - 4,
        "\"target player draws four cards\" — four off the top of the targeted \
         seat's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand + 4,
        "and the four are in that player's hand, so a library that merely \
         emptied would not satisfy the count above"
    );
    assert_eq!(
        library_size(&engine, p0),
        mine,
        "`PlayerRel::Chosen`: the caster's library never moved — the four cards \
         belong to the player the spell named and not to the one who paid for it"
    );
    assert!(
        in_graveyard(&engine, p0, opportunity()).is_some(),
        "the resolved instant is in its owner's graveyard"
    );
}
