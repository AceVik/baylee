#[allow(clippy::wildcard_imports)]
use super::*;

fn curfew() -> CardIndex {
    card_index("6ff1e793-1569-4887-b6bb-d9855c5e8a26")
}

#[test]
fn curfew_returns_a_creature_from_each_player_to_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[curfew()])
        .battlefield(0, &[island(), grizzly_bears(), llanowar_elves()])
        .battlefield(1, &[wild_elephant(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, curfew());
    while !stack_is_empty(&engine) || matches!(engine.pending(), Pending::ChooseCards { .. }) {
        if let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending().clone()
        {
            assert_eq!((min, max), (1, 1));
            assert!(
                options
                    .iter()
                    .all(|id| engine.state().object(*id).unwrap().controller == player)
            );
            let card = if player == p0 {
                grizzly_bears()
            } else {
                wild_elephant()
            };
            let chosen = on_battlefield(&engine, player, card).unwrap();
            assert!(options.contains(&chosen));
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![chosen],
                    },
                )
                .unwrap();
        } else {
            pass_until(&mut engine, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
            });
        }
    }

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(on_battlefield(&engine, p1, wild_elephant()).is_none());
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert!(in_hand(&engine, p1, wild_elephant()).is_some());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p0, island()).is_some());
}
