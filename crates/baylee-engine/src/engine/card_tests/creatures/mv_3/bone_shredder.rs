#[allow(clippy::wildcard_imports)]
use super::*;

fn bone_shredder() -> CardIndex {
    card_index("69aa1ac7-17da-4fc9-a3dc-df8a52841f4a")
}

#[test]
fn bone_shredder_destroys_nonartifact_nonblack_creature_on_etb() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[bone_shredder()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, bone_shredder());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
