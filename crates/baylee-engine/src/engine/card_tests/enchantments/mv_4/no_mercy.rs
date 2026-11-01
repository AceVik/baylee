#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn no_mercy() -> CardIndex {
    card_index("b9538d53-480b-481a-abbf-83ab17e1a45b")
}

#[test]
fn no_mercy_destroys_creature_that_deals_damage_to_you() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[no_mercy()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, baylee_core::ids::Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[0].life, 18);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}

#[test]
fn no_mercy_does_not_destroy_creatures_that_damage_another_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[no_mercy(), gray_ogre()], &[], &[], &[]);
    let ogre = lk::obj(&engine, p0, gray_ogre());
    lk::attack(&mut engine, &[ogre]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, p1), 18);
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}

#[test]
fn no_mercy_also_destroys_a_creature_that_deals_noncombat_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let sorcerer = baylee_core::generated::index::PRODIGAL_SORCERER;
    let mut engine = lk::game(SEED, &[no_mercy()], &[sorcerer], &[], &[]);
    lk::pass_once(&mut engine);
    activate(&mut engine, p1, sorcerer, 0);
    lk::aim(&mut engine, &[], &[p0]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, p0), 19);
    assert!(in_graveyard(&engine, p1, sorcerer).is_some());
}
