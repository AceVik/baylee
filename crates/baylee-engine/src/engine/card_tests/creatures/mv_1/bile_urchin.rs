//! `cards/creatures/mv_1/bile_urchin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bile Urchin — {B}, a 1/1 Spirit: "Sacrifice this creature: Target player
/// loses 1 life."
///
/// One activation plays both halves of the sentence and pins the order the
/// rules put them in: the target is named first (CR 601.2c), so the creature
/// that is about to be paid away is still standing while the question is
/// open, and the sacrifice is the last step (CR 601.2h), so it is in the
/// graveyard before the ability on the stack ever resolves. The opponent is
/// the target on purpose and the two life totals are read separately — a
/// card that drained its own controller, or every player at once, would leave
/// exactly the same single Urchin in the graveyard.
#[test]
fn bile_urchin_sacrifices_itself_to_take_a_life_from_the_player_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[bile_urchin()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let urchin = on_battlefield(&engine, p0, bile_urchin()).expect("the Urchin is on the table");
    assert_eq!(pt(&engine, urchin), (1, 1), "the body the card prints");

    activate(&mut engine, p0, bile_urchin(), 0);
    assert!(
        on_battlefield(&engine, p0, bile_urchin()).is_some(),
        "CR 601.2c comes before CR 601.2h: the creature being paid away is \
         still on the battlefield while its target is chosen"
    );

    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the activating seat names the target");
            assert!(
                options.contains(&p0) && options.contains(&p1),
                "\"target player\" is any player, this one included: {options:?}"
            );
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        Pending::ChooseTargets {
            player,
            player_options,
            min,
            max,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat names the target");
            assert_eq!((min, max), (1, 1), "\"target player\", exactly one");
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" is any player, this one included: {player_options:?}"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        other => panic!("the ability targets a player, got {other:?}"),
    }

    assert!(
        on_battlefield(&engine, p0, bile_urchin()).is_none(),
        "`SacrificeSelf` was paid as the ability was activated"
    );
    assert!(
        in_graveyard(&engine, p0, bile_urchin()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard, not into exile"
    );
    assert!(
        !stack_is_empty(&engine),
        "it is no mana ability: the ability is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "the player that was named lost the one life"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nobody else did — the loss belongs to the target, not to the table"
    );
}
