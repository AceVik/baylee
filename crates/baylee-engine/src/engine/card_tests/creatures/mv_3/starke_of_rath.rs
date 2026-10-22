//! `cards/creatures/mv_3/starke_of_rath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn starke_of_rath() -> CardIndex {
    card_index("07bc407e-7ba1-45aa-a562-0597ebe56044")
}

/// `Starke of Rath` ({1}{R}{R}, 2/2 Legendary Creature — Human Rogue,
/// `Coverage::Implemented`):
/// `{T}: Destroy target artifact or creature. That permanent's controller gains
/// control of Starke.`
#[test]
fn starke_of_rath_destroys_target_creature_and_gives_its_controller_control_of_starke() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[starke_of_rath()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    activate(&mut engine, p0, starke_of_rath(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, starke_of_rath()).is_some());
}
