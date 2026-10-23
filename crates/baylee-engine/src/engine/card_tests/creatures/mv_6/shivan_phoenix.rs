#[allow(clippy::wildcard_imports)]
use super::*;

fn shivan_phoenix() -> CardIndex {
    card_index("49ca8516-624b-4aa8-8e13-d145590cdfda")
}

#[test]
fn shivan_phoenix_returns_to_hand_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), shivan_phoenix()])
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let phoenix = on_battlefield(&engine, p0, shivan_phoenix()).unwrap();
    for _ in 0..2 {
        cast_from_hand(&mut engine, p0, lightning_bolt());
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![phoenix],
                        players: vec![],
                    },
                )
                .unwrap();
        }
        pass_until(&mut engine, stack_is_empty);
    }

    assert!(in_hand(&engine, p0, shivan_phoenix()).is_some());
    assert!(in_graveyard(&engine, p0, shivan_phoenix()).is_none());
}
