//! `cards/artifacts/mv_3/blasting_station.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn blasting_station() -> CardIndex {
    card_index("3a38d2d1-c4ff-4088-b1df-5feb9602ee2e")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Drives a Blasting Station activation: chooses target (object or player) and
/// sacrifices `fodder`, returning the sacrifice menu offered.
#[track_caller]
fn activate_blasting(
    engine: &mut Engine<RegistryLookup>,
    fodder: ObjectId,
    target_objects: &[ObjectId],
    target_players: &[PlayerId],
) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    activate(engine, p0, blasting_station(), 0);
    let mut sac_menu = Vec::new();
    let mut aimed = false;
    for _ in 0..10 {
        if aimed && !sac_menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: target_objects.to_owned(),
                            players: target_players.to_owned(),
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            Pending::ChooseCards {
                player,
                options,
                prompt,
                ..
            } => {
                assert_eq!(prompt, ChoicePrompt::CostSacrifice);
                sac_menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected during Blasting Station activation: {other:?}"),
        }
    }
    assert!(aimed && !sac_menu.is_empty());
    pass_until(engine, stack_is_empty);
    sac_menu
}

/// Blasting Station (3a38d2d1-c4ff-4088-b1df-5feb9602ee2e) — {3} Artifact:
/// "{T}, Sacrifice a creature: This artifact deals 1 damage to any target."
/// "Whenever a creature enters, you may untap this artifact."
///
/// Verifies sacrifice menu (own creatures only), 1 damage to a creature and to
/// a player, and the optional untap trigger on both your and opponent's ETBs
/// (both accepting and declining).
#[test]
fn blasting_station_sacrifices_for_damage_and_untaps_on_creature_etb() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                blasting_station(),
                grizzly_bears(),
                llanowar_elves(),
                forest(),
            ],
        )
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), forest(), forest()])
        .hand(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let station = on_battlefield(&engine, p0, blasting_station()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // 1. Activate targeting opponent's 1/1 Llanowar Elves, sacrificing own Elves.
    let menu = activate_blasting(&mut engine, my_elves, &[their_elves], &[]);
    assert!(menu.contains(&my_elves) && menu.contains(&bears));
    assert!(
        !menu.contains(&station) && !menu.contains(&their_elves),
        "only creatures you control may be sacrificed: {menu:?}"
    );
    assert!(is_tapped(&engine, station));
    assert_eq!(
        engine.state().object(my_elves).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(their_elves).unwrap().zone,
        Zone::Graveyard,
        "1 damage killed opponent's 1/1 Elves"
    );

    // 2. Cast your own creature -> ETB trigger offers YesNo -> accept -> untaps!
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, station),
        "accepted untap on own creature ETB"
    );

    // 3. Activate again targeting opponent p1 directly, sacrificing the new Elves.
    let new_elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    activate_blasting(&mut engine, new_elves, &[], &[p1]);
    assert!(is_tapped(&engine, station));
    assert_eq!(life_of(&engine, p1), 19, "dealt 1 damage to opponent");

    // 4. Opponent casts two creatures: decline first untap (stays tapped), accept second (untaps).
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, station),
        "declined untap keeps Station tapped"
    );

    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, station),
        "opponent's creature ETB can untap Station"
    );
}
