//! `cards/enchantments/auras/mv_3/false_demise.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn false_demise() -> CardIndex {
    card_index("50ffa9f0-fb3b-4de4-b463-6d7ee8b5e93a")
}

#[test]
fn false_demise_returns_enchanted_opponent_creature_under_your_control_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[false_demise(), lightning_bolt()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, false_demise());
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

    cast_with_floating(&mut engine, p0, lightning_bolt());
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

    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "False Demise should return the dead enchanted creature to the battlefield under p0's control"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "p1 should no longer control Grizzly Bears"
    );
}

/// "When enchanted creature dies": a second creature of the same player dies first and stays dead,
/// and only when the enchanted one dies does it come back, under the Aura's controller, out of its
/// owner's graveyard.
#[test]
fn false_demise_returns_only_the_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[false_demise(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, false_demise());
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

    // The unenchanted Gray Ogre dies and nothing brings it back.
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());

    // The enchanted Grizzly Bears dies and comes back for player 0 alone.
    cast_with_floating(&mut engine, p0, lightning_bolt());
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
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_none(),
        "the card left its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, false_demise()).is_none(),
        "the Aura went to the graveyard"
    );
}
