#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn cessation() -> CardIndex {
    card_index("a8b3a35e-0172-4f7b-b50f-571e59d7ca5f")
}

#[test]
fn cessation_stops_enchanted_creature_from_attacking_and_returns_to_hand_when_destroyed() {
    let p0 = PlayerId::new(0);
    let disenchant = baylee_core::generated::index::DISENCHANT;
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[cessation(), disenchant])
        .battlefield(
            0,
            &[
                grizzly_bears(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, cessation());
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

    let cess = on_battlefield(&engine, p0, cessation()).unwrap();
    cast_from_hand(&mut engine, p0, disenchant);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![cess],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, cessation()).is_none());
    assert!(in_hand(&engine, p0, cessation()).is_some());
}

#[test]
fn cessation_removes_only_the_enchanted_creature_from_attackers() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[grizzly_bears(), gray_ogre()],
        &[],
        &[cessation()],
        &[],
    );
    let bear = lk::obj(&engine, p0, grizzly_bears());
    let ogre = lk::obj(&engine, p0, gray_ogre());
    lk::float(&mut engine, p0, &[(ManaColor::White, 3)]);
    lk::cast_at(&mut engine, p0, cessation(), &[bear], &[]);
    let attackers = lk::attackers_offered(&mut engine);
    assert!(!attackers.contains(&bear));
    assert!(attackers.contains(&ogre));
}
