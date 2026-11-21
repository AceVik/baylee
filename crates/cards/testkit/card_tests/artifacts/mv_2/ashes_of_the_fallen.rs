//! `cards/artifacts/mv_2/ashes_of_the_fallen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ashes_of_the_fallen() -> CardIndex {
    card_index("6437867b-a658-4a71-a8a5-0c5a782d1f84")
}

fn lin_sivvi() -> CardIndex {
    card_index("6c3847b6-9bfb-49b5-8f1a-bdb40e6900f1")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Ashes of the Fallen (6437867b-a658-4a71-a8a5-0c5a782d1f84) — {2} Artifact:
/// "As this artifact enters, choose a creature type."
/// "Each creature card in your graveyard has the chosen creature type in
///  addition to its other types."
///
/// Chooses Rebel as Ashes enters, giving Grizzly Bears in YOUR graveyard the
/// Rebel subtype in addition to Bear (proven live by Lin Sivvi targeting it in
/// your graveyard), while excluding noncreature spells in your graveyard and
/// creatures in the opponent's graveyard.
#[test]
fn ashes_of_the_fallen_adds_chosen_creature_type_to_own_graveyard_creatures_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                lin_sivvi(),
                grizzly_bears(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[ashes_of_the_fallen(), lightning_bolt()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put Grizzly Bears in both graveyards and Lightning Bolt in p0's graveyard.
    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    bury(&mut engine, &[my_bears, their_bears]);
    let my_bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let my_mountain = on_battlefield(&engine, p0, mountain()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_mountain);
    cast_with_floating(&mut engine, p0, lightning_bolt());
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

    // Cast Ashes of the Fallen ({2}) and choose Rebel as it enters, keeping 3 Plains for Lin Sivvi.
    let my_plains = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| my_plains[..2].contains(&id));
    cast_with_floating(&mut engine, p0, ashes_of_the_fallen());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::REBEL),
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let rebel = baylee_core::generated::subtypes::creature::REBEL;
    let bear = baylee_core::generated::subtypes::creature::BEAR;
    let my_bears_subtypes = engine
        .state()
        .object(my_bears)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        my_bears_subtypes.contains(rebel) && my_bears_subtypes.contains(bear),
        "own graveyard creature gains Rebel in addition to Bear"
    );
    assert!(
        !engine
            .state()
            .object(my_bolt)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(rebel),
        "noncreature card in your graveyard does not gain the chosen creature type"
    );
    assert!(
        !engine
            .state()
            .object(their_bears)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(rebel),
        "opponent's graveyard creature does not gain the chosen creature type"
    );

    // Live gameplay proof: Lin Sivvi's ability 1 ("{3}: Put target Rebel card from
    // your graveyard on the bottom of your library") now legally targets Grizzly Bears.
    tap_mana_where(&mut engine, p0, |id| my_plains[2..5].contains(&id));
    activate(&mut engine, p0, lin_sivvi(), 1);
    let menu = aim_at(&mut engine, p0, my_bears);
    assert!(
        menu.contains(&my_bears),
        "Grizzly Bears is a legal Rebel target"
    );
    assert!(!menu.contains(&my_bolt) && !menu.contains(&their_bears));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .first()
            .copied(),
        Some(my_bears)
    );
}
