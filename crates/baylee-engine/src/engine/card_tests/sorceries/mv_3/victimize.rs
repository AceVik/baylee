#[allow(clippy::wildcard_imports)]
use super::*;

fn victimize() -> CardIndex {
    card_index("240e85d3-e495-4877-8609-4b4056c402f7")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn victimize_sacrifices_creature_to_return_two_creatures_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[victimize(), gray_ogre(), grizzly_bears()])
        .battlefield(0, &[llanowar_elves(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf on bf");
    let ogre = in_hand(&engine, p0, gray_ogre()).expect("ogre in hand");
    let bears = in_hand(&engine, p0, grizzly_bears()).expect("bears in hand");
    {
        let state = engine.dev_state_mut(p0).unwrap();
        state
            .move_object(
                ogre,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Setup,
            )
            .unwrap();
        state
            .move_object(
                bears,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Setup,
            )
            .unwrap();
    }
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, victimize());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre, bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
