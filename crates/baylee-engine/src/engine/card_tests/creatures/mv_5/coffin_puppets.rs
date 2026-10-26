//! `cards/creatures/mv_5/coffin_puppets.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn coffin_puppets() -> CardIndex {
    card_index("e49771ab-cabb-4a5f-aef3-a0ac8147e6bf")
}

#[test]
fn coffin_puppets_returns_from_graveyard_during_your_upkeep_if_you_control_a_swamp() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[coffin_puppets()])
        .battlefield(0, &[swamp(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, coffin_puppets());

    // Cannot activate during main phase.
    let puppets = in_graveyard(&engine, p0, coffin_puppets()).unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: puppets,
                    ability_index: 0,
                },
            )
            .is_err()
    );

    // Advance to p0's next upkeep.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::Upkeep
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let s = on_battlefield(&engine, p0, swamp()).unwrap();
    let f = on_battlefield(&engine, p0, forest()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: puppets,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![s] })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![f] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, coffin_puppets()).is_some());
    assert!(in_graveyard(&engine, p0, swamp()).is_some());
    assert!(in_graveyard(&engine, p0, forest()).is_some());
}

#[test]
fn coffin_puppets_cannot_activate_without_a_swamp() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[coffin_puppets()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, coffin_puppets());

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::Upkeep
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let puppets = in_graveyard(&engine, p0, coffin_puppets()).unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: puppets,
                    ability_index: 0,
                },
            )
            .is_err(),
        "Coffin Puppets requires controlling a Swamp"
    );
}
