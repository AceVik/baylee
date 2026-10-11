//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn carpet_prevents_only_attacking_ground_sources_only_to_its_controller() {
    use baylee_core::ids::Defender;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::AL_ABARA_S_CARPET])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    let flyer = on_battlefield(&e, p1, index::AIR_ELEMENTAL).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::ChooseAttackers {player,..} if *player==p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p0)), (flyer, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority {player,..} if *player==p0),
    );
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 5);
    e.refresh_offer();
    activate(&mut e, p0, index::AL_ABARA_S_CARPET, 0);
    pass_until(&mut e, stack_is_empty);
    for (source, recipient, combat, expected) in [
        (bear, p0, false, 0),
        (bear, p0, true, 0),
        (bear, p1, false, 1),
        (flyer, p0, true, 1),
    ] {
        let state = e.dev_state_mut(p0).unwrap();
        let mut work = crate::damage::DamageWork::new(
            state,
            vec![crate::damage::Assignment {
                source,
                source_version: Some(state.object(source).unwrap().version),
                recipient: crate::event::DamageTarget::Player(recipient),
                amount: 1,
                is_combat: combat,
            }],
        );
        assert!(work.advance(state).is_none());
        assert_eq!(work.dealt(), expected);
    }
}
