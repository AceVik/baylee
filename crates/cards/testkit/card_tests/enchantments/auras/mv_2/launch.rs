#[allow(clippy::wildcard_imports)]
use super::*;

fn launch() -> CardIndex {
    card_index("9e3b373f-05f5-44df-9568-bd1f8039aa54")
}

#[test]
fn launch_grants_flying_and_returns_to_hand_when_put_into_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[launch()])
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, launch());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        let options = aim_at(&mut engine, p0, bear);
        let land = on_battlefield(&engine, p0, island()).unwrap();
        assert!(!options.contains(&land));
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bear).contains(KeywordSet::FLYING));
    let aura = on_battlefield(&engine, p0, launch()).expect("attached");
    kill(&mut engine, aura);

    assert!(in_hand(&engine, p0, launch()).is_some());
}
