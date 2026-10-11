//! `cards/creatures/mv_4/urza_lord_high_artificer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza, Lord High Artificer — {2}{U}{U} 1/4 — the two printed sentences that
/// exercise entry and mana production in one first main phase. The entry trigger
/// makes a 0/0 Construct *artifact creature* whose own static grows it by one
/// for each artifact its controller controls, so the token standing beside the
/// Sol Ring has to be a 2/2: one would mean it never counted itself, three that
/// the opponent's Sol Ring across the table was counted too. And "Tap an
/// untapped artifact you control: Add {U}" is a mana ability whose cost names
/// no artifact of its own, so the question, the menu it publishes and the {U}
/// it pays are all read off the same activation.
#[allow(clippy::too_many_lines)] // the token's body read off the board, then its mana ability
#[test]
fn urza_builds_a_construct_that_counts_your_artifacts_and_taps_one_for_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), quiet_artifact()],
        )
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[urza_lord_high_artificer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Islands pay {2}{U}{U}. The Sol Ring is kept back through both
    // taps below: it is the "untapped artifact you control" Urza's own mana
    // ability charges, and it is what the Construct counts.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    cast_with_floating(&mut engine, p0, urza_lord_high_artificer());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && !tokens_of(e, p0).is_empty()
    });

    let urza = on_battlefield(&engine, p0, urza_lord_high_artificer()).expect("Urza resolved");
    assert_eq!(pt(&engine, urza), (1, 4), "the body the card prints");

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one Construct — a token left at 0/0 would have died to CR 704.5f"
    );
    let construct = tokens[0];
    let kinds = types(&engine, construct);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "the token counts itself because it is an artifact creature: {kinds:?}"
    );
    assert_eq!(
        pt(&engine, construct),
        (2, 2),
        "+1/+1 for each artifact *you* control: the Sol Ring and the Construct \
         itself, and nothing for the Sol Ring the opponent controls"
    );

    // "Tap an untapped artifact you control: Add {U}". Tapping everything
    // else first settles the pool: a mana ability needs no mana, but the
    // offer is read once nothing else is floating.
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(urza, 1)),
        "there is an untapped artifact to tap, so the printed mana ability is \
         offered: {:?}",
        legal.abilities
    );
    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Blue);

    activate(&mut engine, p0, urza_lord_high_artificer(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the tap is chosen before it is paid, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, and the cost asks once");
    assert!(
        options.contains(&ring),
        "the untapped artifact you control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an artifact this seat does not control is not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact the question offered pays the cost");

    assert!(is_tapped(&engine, ring), "the artifact that was named paid");
    assert!(
        !is_tapped(&engine, construct),
        "and the Construct beside it never moved"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        before + 1,
        "{{U}} reached the pool the moment the answer landed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        u64::from(before) + 1,
        "one mana, off one tap, and nothing else came with it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}

#[test]
fn urza_shuffles_exiles_and_waives_mana_costs_but_keeps_land_limits() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for card_type in [index::ORNITHOPTER, index::BLACKBLOOM_ROGUE, index::FOREST] {
        let mut e = Duel::new(SEED, card_type)
            .battlefield(0, &[urza_lord_high_artificer()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let before = e.state().zones.list(ZoneLocation::Library(p0)).clone();
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
        e.refresh_offer();
        activate(&mut e, p0, urza_lord_high_artificer(), 2);
        pass_until(&mut e, stack_is_empty);
        let card = *e
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .last()
            .unwrap();
        assert_eq!(
            e.state().zones.list(ZoneLocation::Library(p0)).len() + 1,
            before.len()
        );
        assert_ne!(
            e.state().zones.list(ZoneLocation::Library(p0)),
            &before[..before.len() - 1],
            "library shuffled"
        );
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        if card_type == index::FOREST {
            assert!(priority_offer(&e).lands.contains(&card));
            e.apply(p0, PlayerAction::PlayLand { card }).unwrap();
            assert_eq!(e.state().players[0].lands_played_this_turn, 1);
            e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
            e.refresh_offer();
            activate(&mut e, p0, urza_lord_high_artificer(), 2);
            pass_until(&mut e, stack_is_empty);
            let second = *e
                .state()
                .zones
                .list(ZoneLocation::Exile(p0))
                .last()
                .unwrap();
            assert!(!priority_offer(&e).lands.contains(&second));
        } else {
            assert!(priority_offer(&e).castable.contains(&card));
            if card_type == index::BLACKBLOOM_ROGUE {
                assert!(priority_offer(&e).lands.contains(&card));
            }
            cast_object_and_resolve(&mut e, p0, card);
        }
    }
}

#[test]
fn urza_free_adventure_can_choose_either_spell_and_a_single_instant_option_stays_that_face() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for instant_only in [false, true] {
        let mut e = Duel::new(SEED, index::TWINING_TWINS)
            .battlefield(0, &[urza_lord_high_artificer(), grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
        e.refresh_offer();
        activate(&mut e, p0, urza_lord_high_artificer(), 2);
        pass_until(&mut e, stack_is_empty);
        let card = *e
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .last()
            .unwrap();
        if instant_only {
            // A priority opportunity outside a main phase; no alternative-cost timing waiver.
            e.state.turn.phase = crate::turn::Phase::Ending;
            e.state.turn.step = crate::turn::Step::End;
            e.refresh_offer();
        }
        e.apply(p0, PlayerAction::CastSpell { card }).unwrap();
        if !instant_only {
            let Pending::ChooseCastMode { options, .. } = e.pending() else {
                panic!("{:?}", e.pending())
            };
            assert!(
                options
                    .iter()
                    .any(|o| o.kind == crate::choice::CastModeKind::Normal)
            );
            let which = options
                .iter()
                .position(|o| o.kind == crate::choice::CastModeKind::Face(1))
                .unwrap();
            e.apply(p0, PlayerAction::ChooseMode(which)).unwrap();
        }
        let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        aim_at(&mut e, p0, bear);
        assert_eq!(e.state().object(card).unwrap().face_index, 1);
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
    }
}

#[test]
fn urza_free_adventure_tax_cannot_use_mana_restricted_to_its_creature_front() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for pay_tax in [false, true] {
        let mut e = Duel::new(SEED, index::TWINING_TWINS)
            .battlefield(
                0,
                &[
                    urza_lord_high_artificer(),
                    grizzly_bears(),
                    index::GLOOM,
                    index::ANCIENT_ZIGGURAT,
                    index::ANCIENT_ZIGGURAT,
                    index::ANCIENT_ZIGGURAT,
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
        e.refresh_offer();
        activate(&mut e, p0, urza_lord_high_artificer(), 2);
        pass_until(&mut e, stack_is_empty);
        let card = *e
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .last()
            .unwrap();
        let ziggurats: Vec<_> = e
            .state()
            .battlefield_seen()
            .filter(|&id| {
                e.state()
                    .object(id)
                    .unwrap()
                    .card
                    .is_some_and(|c| c.index == index::ANCIENT_ZIGGURAT)
            })
            .collect();
        for source in ziggurats {
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
            )
            .unwrap();
            e.apply(p0, PlayerAction::ChooseColor(ManaColor::White))
                .unwrap();
        }
        if pay_tax {
            e.state.players[0].mana_pool.add(ManaColor::Colorless, 3);
            e.refresh_offer();
        }
        e.apply(p0, PlayerAction::CastSpell { card }).unwrap();
        if pay_tax {
            let Pending::ChooseCastMode { options, .. } = e.pending() else {
                panic!("{:?}", e.pending())
            };
            let which = options
                .iter()
                .position(|o| o.kind == crate::choice::CastModeKind::Face(1))
                .unwrap();
            e.apply(p0, PlayerAction::ChooseMode(which)).unwrap();
            let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
            aim_at(&mut e, p0, bear);
            assert_eq!(e.state().object(card).unwrap().face_index, 1);
        } else {
            assert_eq!(e.state().object(card).unwrap().face_index, 0);
        }
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Stack);
        assert_eq!(
            e.state().players[0].mana_pool.total(),
            3,
            "creature-only mana stays unspent"
        );
    }
}
