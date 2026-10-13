//! `cards/creatures/mv_2/goblin_engineer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Engineer's activated half — "{R}, {T}, Sacrifice an artifact:
/// Return target artifact card with mana value 3 or less from your graveyard
/// to the battlefield". Three readings are struck by one activation: the
/// card that comes back is a Chromatic Lantern, mana value 3 exactly, so a
/// filter that read "less than three" would leave the ability with no target
/// at all; the identical Lantern in the opponent's graveyard is the "your"
/// the sentence prints; and the menu the sacrifice asks with holds the Sol
/// Ring this seat controls rather than the one across the table. Tapping to
/// float the {R} first is not scenery either — the ability is only offered
/// once the cost is payable, and it ends tapped with the red gone.
#[allow(clippy::too_many_lines)] // a sacrifice, a graveyard search and the permanent that arrives tapped
#[test]
fn goblin_engineer_trades_an_artifact_for_the_lantern_in_his_own_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The backing deck is the card's own graveyard clause: nothing else in the
    // harness puts a named *artifact card* into a graveyard — an opening
    // battlefield holds permanents and `seed_graveyard` moves cards off the top
    // of a library — so the library has to be the printing the ability is
    // about. Chromatic Lantern is mana value 3 exactly, the number "3 or less"
    // has to include.
    let mut engine = Duel::new(SEED, chromatic_lantern())
        .battlefield(0, &[goblin_engineer(), mountain(), quiet_artifact()])
        // The same card and the same artifact across the table, so each
        // refusal is made by the sentence rather than by the board.
        .battlefield(1, &[forest(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    let engineer = on_battlefield(&engine, p0, goblin_engineer()).expect("the Engineer is out");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let lantern =
        in_graveyard(&engine, p0, chromatic_lantern()).expect("my Lantern is in my graveyard");
    let their_lantern =
        in_graveyard(&engine, p1, chromatic_lantern()).expect("theirs is in their graveyard");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain is floating the {{R}} the cost asks for"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(engineer, 0)),
        "a card in the graveyard is a target and a red source is floating, so \
         the one line the Engineer prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, goblin_engineer(), 0);
    // CR 601.2c comes before CR 601.2h: the graveyard is asked about first and
    // the sacrifice second.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&lantern),
        "mana value 3 is within \"3 or less\": {options:?}"
    );
    assert!(
        !options.contains(&their_lantern),
        "\"from your graveyard\" — the identical card across the table is not \
         offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lantern],
            },
        )
        .expect("the card the question offered is the card it takes");

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the artifact this seat controls is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&their_rock),
        "CR 701.21a: an opponent's artifact is not yours to sacrifice: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is the whole menu — the Engineer is a creature and the Mountain \
         a land"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, chromatic_lantern()).is_some(),
        "the ability returns the card from the graveyard to the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, chromatic_lantern()).is_none(),
        "and it is no longer a card in a graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the sacrificed artifact is what the ability spent"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, chromatic_lantern()).is_some(),
        "the other seat's graveyard was never read, let alone moved from"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and neither was their artifact"
    );
    assert!(is_tapped(&engine, engineer), "{{T}} was part of the cost");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "and the {{R}} was spent, not merely floated"
    );
}

#[test]
fn engineer_may_decline_without_shuffling_or_search_an_artifact_into_its_graveyard() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for accept in [false, true] {
        let mut e = Duel::new(SEED, index::MIND_STONE)
            .hand(0, &[goblin_engineer()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Red, 2);
        e.refresh_offer();
        let library = e.state().zones.list(ZoneLocation::Library(p0)).clone();
        cast_with_floating(&mut e, p0, goblin_engineer());
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::MayDo,
                    ..
                }
            )
        });
        e.apply(p0, PlayerAction::YesNo(accept)).unwrap();
        if accept {
            let Pending::ChooseCards {
                options, prompt, ..
            } = e.pending().clone()
            else {
                panic!("search")
            };
            assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
            let found = options[0];
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![found],
                },
            )
            .unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(found).unwrap().zone, Zone::Graveyard);
            assert_eq!(
                e.state().zones.list(ZoneLocation::Library(p0)).len(),
                library.len() - 1
            );
        } else {
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().zones.list(ZoneLocation::Library(p0)), &library);
        }
    }
}

/// "you may search your library for an **artifact card**": with a Forest
/// library and one Mind Stone put into it, the Stone is the whole of what the
/// search offers; the sixty Forests behind it are not.
#[test]
fn engineer_search_offers_only_artifact_cards() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[goblin_engineer(), index::MIND_STONE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let stone = in_hand(&e, p0, index::MIND_STONE).expect("the Stone starts in hand");
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            stone,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 2);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, goblin_engineer());
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCards {
        options, prompt, ..
    } = e.pending().clone()
    else {
        panic!("the search asks which card: {:?}", e.pending())
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
    assert_eq!(
        options,
        vec![stone],
        "\"an artifact card\": the Mind Stone alone, none of the Forests"
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![stone],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, p0, index::MIND_STONE).is_some(),
        "the Stone is put into the graveyard, not the hand"
    );
}

/// "Return target artifact card with mana value **3 or less**": a Jayemdae
/// Tome (mana value 4) in the graveyard is no target, so alone it leaves the
/// ability unoffered; beside a Chromatic Lantern (mana value 3) the menu is the
/// Lantern and not the Tome.
#[test]
fn engineer_reanimation_leaves_a_mana_value_four_artifact_in_the_graveyard() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, chromatic_lantern())
        .battlefield(0, &[goblin_engineer(), mountain(), quiet_artifact()])
        .hand(0, &[index::JAYEMDAE_TOME])
        .start();
    keep_mulligans(&mut e);
    assert!(
        walk_to_own_main(&mut e, p0),
        "p0 reaches its own first main phase"
    );
    let engineer = on_battlefield(&e, p0, goblin_engineer()).expect("the Engineer is out");
    let tome = in_hand(&e, p0, index::JAYEMDAE_TOME).expect("the Tome starts in hand");
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            tome,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    tap_all_mana(&mut e, p0);
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("priority, got {:?}", e.pending())
    };
    assert!(
        !legal.abilities.contains(&(engineer, 0)),
        "mana value 4 is not \"3 or less\": the Tome is the only artifact card in the graveyard \
         and the {{R}} is floating, so the ability has no target and is not offered"
    );

    seed_graveyard(&mut e, p0, 1);
    let lantern = in_graveyard(&e, p0, chromatic_lantern()).expect("a Lantern is in the graveyard");
    activate(&mut e, p0, goblin_engineer(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the ability targets: {:?}", e.pending())
    };
    assert_eq!(
        options,
        vec![lantern],
        "mana value 3 qualifies and mana value 4 does not"
    );
}
