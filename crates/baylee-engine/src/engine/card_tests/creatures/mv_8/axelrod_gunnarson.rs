//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn axelrod_death_trigger_gains_one_and_damages_only_its_legal_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::AXELROD_GUNNARSON])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, index::AXELROD_GUNNARSON).unwrap();
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    let mut damage = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source,
            source_version: Some(state.object(source).unwrap().version),
            recipient: crate::event::DamageTarget::Object(bear),
            amount: 1,
            is_combat: false,
        }],
    );
    assert!(damage.advance(state).is_none());
    bury(&mut e, &[bear]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert!(options.is_empty());
    assert_eq!(player_options, &vec![p0, p1]);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 21);
    assert_eq!(e.state().players[1].life, 19);
}
