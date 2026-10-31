//! Mirror Golem gains protection from the imprinted card's types, including damage.

use super::*;

fn mirror_golem() -> CardIndex {
    card_index("3460e86f-c182-4ca9-bd90-7949b0759c98")
}

#[test]
fn mirror_golem_prevents_combat_damage_from_the_imprinted_creature_type() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 6])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[mirror_golem(), gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let copy = in_hand(&e, p0, gray_ogre()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            copy,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    cast_from_hand(&mut e, p0, mirror_golem());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![copy],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(copy).unwrap().zone, Zone::Exile);

    let golem = on_battlefield(&e, p0, mirror_golem()).unwrap();
    let attacker = on_battlefield(&e, p1, gray_ogre()).unwrap();
    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(golem, attacker)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(e.state().object(golem).unwrap().damage, 0);
    assert!(in_graveyard(&e, p1, gray_ogre()).is_some());
}
