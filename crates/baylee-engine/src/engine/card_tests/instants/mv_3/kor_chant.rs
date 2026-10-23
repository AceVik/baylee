//! `cards/instants/mv_3/kor_chant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn kor_chant() -> CardIndex {
    card_index("5b3f6817-5d7a-4d83-ad1d-df75b4e1970b")
}

#[test]
fn kor_chant_prevents_combat_damage_to_target_creature_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[gray_ogre(), plains(), plains(), plains()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[kor_chant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let my_bears = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let opp_bears = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, kor_chant());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[my_bears], &[])
    });

    kit::declare_attack(&mut engine, &[my_bears], p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(opp_bears, my_bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}
