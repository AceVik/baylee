#[allow(clippy::wildcard_imports)]
use super::*;

fn show_and_tell() -> CardIndex {
    card_index("b83a3ba0-249e-4c39-bbf0-cb005413f7d2")
}

#[test]
fn show_and_tell_puts_creature_from_hand_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[show_and_tell(), gray_ogre()])
        .battlefield(0, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = in_hand(&engine, p0, gray_ogre()).expect("ogre in hand");
    cast_from_hand(&mut engine, p0, show_and_tell());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    while matches!(engine.pending(), Pending::ChooseCards { .. }) {
        let Pending::ChooseCards {
            player, options, ..
        } = engine.pending().clone()
        else {
            break;
        };
        if player == p0 && options.contains(&bear_id) {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![bear_id],
                    },
                )
                .unwrap();
        } else {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: Vec::new(),
                    },
                )
                .unwrap();
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
