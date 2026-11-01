#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

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

#[test]
fn bone_shredder_excludes_black_and_artifact_creatures_then_owes_echo() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let black = baylee_core::generated::index::BLACK_KNIGHT;
    let artifact = baylee_core::generated::index::WALL_OF_JUNK;
    let mut engine = lk::game(
        SEED,
        &[],
        &[grizzly_bears(), black, artifact],
        &[bone_shredder()],
        &[],
    );
    lk::float(&mut engine, p0, &[(ManaColor::Black, 3)]);
    cast_with_floating(&mut engine, p0, bone_shredder());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&lk::obj(&engine, p1, black)));
    assert!(!options.contains(&lk::obj(&engine, p1, artifact)));
    let bear = lk::obj(&engine, p1, grizzly_bears());
    lk::aim(&mut engine, &[bear], &[]);
    lk::settle(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    assert!(on_battlefield(&engine, p0, bone_shredder()).is_some());
    reach_their_main_phase(&mut engine, p0);
    assert!(in_graveyard(&engine, p0, bone_shredder()).is_some());
}
