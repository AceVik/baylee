//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn glyph_prevents_damage_for_the_turn_then_destroys_its_remembered_wall() {
    use baylee_core::ids::Defender;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::WALL_OF_WOOD, index::MOUNTAIN])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::GLYPH_OF_DESTRUCTION])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let wall = on_battlefield(&e, p0, index::WALL_OF_WOOD).unwrap();
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::ChooseAttackers {player,..} if *player==p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, bear)],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority {player,..} if *player==p0),
    );
    cast_from_hand(&mut e, p0, index::GLYPH_OF_DESTRUCTION);
    aim_at(&mut e, p0, wall);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, wall), (10, 3));
    let state = e.dev_state_mut(p0).unwrap();
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source: bear,
            source_version: Some(state.object(bear).unwrap().version),
            recipient: crate::event::DamageTarget::Object(wall),
            amount: 100,
            is_combat: false,
        }],
    );
    assert!(work.advance(state).is_none());
    assert_eq!(work.dealt(), 0);
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::Ending && stack_is_empty(e)
    });
    assert_eq!(e.state().object(wall).unwrap().zone, Zone::Graveyard);
}
