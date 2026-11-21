//! `cards/artifacts/mv_2/armillary_sphere.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn armillary_sphere() -> CardIndex {
    card_index("3963140c-da67-43e6-9514-fe9dc0a43c4d")
}

fn strip_mine() -> CardIndex {
    card_index("d21a89eb-7c5b-459a-acc7-12b20b13bf79")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Armillary Sphere (3963140c-da67-43e6-9514-fe9dc0a43c4d) — {2} Artifact:
/// "{2}, {T}, Sacrifice this artifact: Search your library for up to two basic
/// land cards, reveal them, put them into your hand, then shuffle."
///
/// Pays {2}, taps and sacrifices Armillary Sphere, excludes nonbasic lands
/// from the search menu, and puts two chosen basic lands into hand.
#[test]
fn armillary_sphere_searches_for_two_basic_lands_and_excludes_nonbasics() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, grizzly_bears())
        .battlefield(0, &[armillary_sphere(), plains(), plains()])
        .hand(0, &[strip_mine(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_library_top(&mut engine, p0, strip_mine());
    hand_to_library_top(&mut engine, p0, island());
    hand_to_library_top(&mut engine, p0, forest());

    let sphere = on_battlefield(&engine, p0, armillary_sphere()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sphere,
                ability_index: 0,
            },
        )
        .expect("activate Armillary Sphere");

    assert!(
        in_graveyard(&engine, p0, armillary_sphere()).is_some(),
        "Armillary Sphere was sacrificed as part of its activation cost"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(min, 0, "searching is optional ('up to two')");
    assert_eq!(max, 2, "may find up to two basic lands");
    assert_eq!(
        options.len(),
        2,
        "only basic lands (Island, Forest) offered; nonbasic Strip Mine and creatures excluded"
    );
    for &id in &options {
        let idx = engine
            .state()
            .object(id)
            .and_then(|o| o.card)
            .unwrap()
            .index;
        assert!(
            idx == island() || idx == forest(),
            "offered card must be one of the two basic lands"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .expect("choose both basic lands");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "Island entered hand"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "Forest entered hand"
    );
    assert!(
        in_hand(&engine, p0, strip_mine()).is_none(),
        "nonbasic Strip Mine stayed in library"
    );
}

/// "Up to two": the player may find zero basic lands (or one), and cannot
/// activate without {2} available.
#[test]
fn armillary_sphere_allows_finding_zero_and_requires_two_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[armillary_sphere(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphere = on_battlefield(&engine, p0, armillary_sphere()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(
        !priority_offer(&engine).abilities.contains(&(sphere, 0)),
        "cannot activate Armillary Sphere with only {{1}}"
    );

    let mut engine2 = Duel::new(SEED, forest())
        .battlefield(0, &[armillary_sphere(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);
    let hand_before = engine2.state().zones.list(ZoneLocation::Hand(p0)).len();
    let sphere2 = on_battlefield(&engine2, p0, armillary_sphere()).unwrap();
    tap_all_mana(&mut engine2, p0);
    engine2
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sphere2,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine2
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("fail to find 0 basic lands");
    pass_until(&mut engine2, stack_is_empty);

    assert_eq!(
        engine2.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "choosing 0 basic lands adds nothing to hand"
    );
    assert!(in_graveyard(&engine2, p0, armillary_sphere()).is_some());
}
