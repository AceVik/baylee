//! `cards/creatures/mv_4/treefolk_mystic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn treefolk_mystic() -> CardIndex {
    card_index("68a6fd00-9cb0-4518-97aa-b7f475da58c1")
}

#[test]
fn treefolk_mystic_destroys_all_auras_attached_to_blocked_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let aura = baylee_core::generated::index::GRANITE_GRIP;
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[aura])
        .battlefield(0, &[gray_ogre(), mountain(), mountain(), mountain()])
        .battlefield(1, &[treefolk_mystic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, aura);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, aura).is_some());

    let mystic = on_battlefield(&engine, p1, treefolk_mystic()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, ogre, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(mystic, ogre)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, aura).is_none());
    assert!(in_graveyard(&engine, p0, aura).is_some());
}

#[test]
fn treefolk_mystic_also_destroys_auras_when_it_is_blocked() {
    use crate::engine::card_tests::legends_kit as lk;
    let aura = baylee_core::generated::index::GRANITE_GRIP;
    let mut engine = lk::game(
        SEED,
        &[treefolk_mystic(), llanowar_elves()],
        &[gray_ogre()],
        &[aura, rancor()],
        &[],
    );
    let mystic = lk::obj(&engine, lk::P0, treefolk_mystic());
    let elf = lk::obj(&engine, lk::P0, llanowar_elves());
    let blocker = lk::obj(&engine, lk::P1, gray_ogre());
    lk::float(
        &mut engine,
        lk::P0,
        &[(ManaColor::Red, 3), (ManaColor::Green, 1)],
    );
    lk::cast_at(&mut engine, lk::P0, aura, &[blocker], &[]);
    lk::cast_at(&mut engine, lk::P0, rancor(), &[elf], &[]);
    lk::attack(&mut engine, &[mystic]);
    lk::block(&mut engine, &[(blocker, mystic)]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, lk::P0, aura).is_some());
    assert!(
        on_battlefield(&engine, lk::P0, rancor()).is_some(),
        "an Aura on an unrelated creature stays"
    );
}
