//! `cards/creatures/mv_3/goblin_sharpshooter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Goblin Sharpshooter (`d81285b7-a718-411a-8be3-ecc0cfe0bcb0`) — {2}{R} Creature — Goblin 1/1:
/// "This creature doesn't untap during your untap step.
/// Whenever a creature dies, untap this creature.
/// {T}: This creature deals 1 damage to any target."
#[test]
fn goblin_sharpshooter_pings_creature_and_untaps_when_it_dies() {
    let sharpshooter_card = card_index("d81285b7-a718-411a-8be3-ecc0cfe0bcb0");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[sharpshooter_card])
        .battlefield(1, &[index::RAGING_GOBLIN])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sharpshooter = on_battlefield(&engine, p0, sharpshooter_card).unwrap();
    let target_goblin = on_battlefield(&engine, p1, index::RAGING_GOBLIN).unwrap();

    // Activate {T}: deal 1 damage to Raging Goblin (1/1).
    activate(&mut engine, p0, sharpshooter_card, 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_goblin],
            },
        )
        .unwrap();
    assert!(
        engine
            .state()
            .object(sharpshooter)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Damage kills Raging Goblin, which triggers Goblin Sharpshooter's untap ability.
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, index::RAGING_GOBLIN).is_some());
    assert!(
        !engine
            .state()
            .object(sharpshooter)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Goblin Sharpshooter untapped when Raging Goblin died"
    );

    // Activate again targeting player 1 directly; this time nothing dies so it stays tapped across p0's next untap step.
    activate(&mut engine, p0, sharpshooter_card, 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(sharpshooter)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Goblin Sharpshooter does not untap during its controller's untap step"
    );
}
