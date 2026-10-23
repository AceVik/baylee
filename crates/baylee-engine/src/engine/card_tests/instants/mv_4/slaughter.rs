#[allow(clippy::wildcard_imports)]
use super::*;

fn slaughter() -> CardIndex {
    card_index("37de575d-15d4-42a2-a3d2-3855fb6ab110")
}

#[test]
fn slaughter_destroys_target_nonblack_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[slaughter()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, slaughter());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
