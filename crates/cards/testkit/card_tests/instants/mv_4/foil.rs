//! `cards/instants/mv_4/foil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "7dde4eb6-e9d7-4259-abc2-3af738e0f00f"
fn foil() -> CardIndex {
    card_index("7dde4eb6-e9d7-4259-abc2-3af738e0f00f")
}

fn hand_len(engine: &Engine<RegistryLookup>, player: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(player)).len()
}

/// Foil ({2}{U}{U} Instant):
/// "You may discard an Island card and another card rather than pay this spell's
/// mana cost. Counter target spell."
///
/// Played both with zero mana available anywhere (Hand = Foil + Island + Grizzly
/// Bears) and by explicitly choosing `CastModeKind::Alternative(0)` from
/// `Pending::ChooseCastMode` while `{2}{U}{U}` is also floating.
#[test]
fn foil_casts_by_discarding_an_island_card_and_another_card_with_no_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // 1. No lands and empty mana pool for P0: Foil is castable solely via its
    // alternative cost, asks for the Island card first and the second card next,
    // pays both before becoming cast (CR 601.2h), spends no mana, and counters
    // P1's creature spell into P1's graveyard (CR 701.6a).
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[foil(), island(), index::GRIZZLY_BEARS])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, index::GRIZZLY_BEARS);
    let bears_spell = on_stack(&engine, index::GRIZZLY_BEARS).expect("Grizzly Bears on stack");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    let foil_card = in_hand(&engine, p0, foil()).expect("Foil in hand");
    let p0_island = in_hand(&engine, p0, island()).expect("Island in hand");
    let p0_bears = in_hand(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears in hand");
    let p1_forest = on_battlefield(&engine, p1, forest()).expect("P1 Forest");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.mana_abilities.is_empty() && legal.abilities.is_empty(),
        "P0 has no mana sources at all"
    );
    assert!(
        legal.castable.contains(&foil_card),
        "Foil is castable through its discard alternative cost with 0 mana: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: foil_card })
        .expect("castable via alternative cost");

    // Target step (CR 601.2c): only spells on the stack are legal targets.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![bears_spell]);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![p1_forest],
                    players: vec![],
                },
            )
            .is_err(),
        "a permanent on the battlefield is not a legal target for Foil"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_spell],
                players: vec![],
            },
        )
        .unwrap();

    // First discard step (CR 601.2h): "an Island card".
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected Island discard choice, got {:?}", engine.pending());
    };
    assert_eq!(
        (player, min, max, prompt),
        (p0, 1, 1, ChoicePrompt::CostDiscard)
    );
    assert_eq!(
        options,
        vec![p0_island],
        "only the Island card in P0's hand satisfies the first discard"
    );
    for illegal in [p0_bears, foil_card] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![illegal],
                    },
                )
                .is_err(),
            "neither a non-Island card nor Foil itself may pay the Island discard"
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_island],
            },
        )
        .unwrap();

    // Second discard step (CR 601.2h): "another card" — neither Foil (CR 601.2a)
    // nor the Island already chosen.
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected second discard choice, got {:?}", engine.pending());
    };
    assert_eq!((min, max, prompt), (1, 1, ChoicePrompt::CostDiscard));
    assert_eq!(options, vec![p0_bears]);
    for illegal in [p0_island, foil_card] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![illegal],
                    },
                )
                .is_err(),
            "neither the already-chosen Island nor Foil itself may pay the second discard"
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_bears],
            },
        )
        .unwrap();

    // Costs are already paid when Foil becomes cast on the stack (CR 601.2h).
    let foil_on_stack = on_stack(&engine, foil()).expect("Foil is on the stack");
    assert!(engine.state().object(foil_on_stack).unwrap().alt_cast);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(hand_len(&engine, p0), 0);
    assert!(in_graveyard(&engine, p0, island()).is_some());
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, foil()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // 2. With `{2}{U}{U}` floating as well as Island + Grizzly Bears in hand,
    // `Pending::ChooseCastMode` offers both `Normal` and `Alternative(0)`;
    // choosing `Alternative(0)` leaves all four floating mana unspent.
    let mut both = Duel::new(SEED + 1, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[foil(), island(), index::GRIZZLY_BEARS])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut both);
    reach_their_main_phase(&mut both, p1);

    cast_from_hand(&mut both, p1, index::GRIZZLY_BEARS);
    let bears_spell = on_stack(&both, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut both,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let held_island = in_hand(&both, p0, island()).unwrap();
    let held_bears = in_hand(&both, p0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut both, p0);
    assert_eq!(both.state().players[0].mana_pool.total(), 4);
    cast_with_floating(&mut both, p0, foil());

    let Pending::ChooseCastMode { options, .. } = both.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", both.pending());
    };
    assert!(
        options
            .iter()
            .any(|o| matches!(o.kind, CastModeKind::Normal))
    );
    let alt = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Alternative(0)))
        .expect("alternative discard cost is offered beside {2}{U}{U}");
    both.apply(p0, PlayerAction::ChooseMode(alt)).unwrap();
    both.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bears_spell],
            players: vec![],
        },
    )
    .unwrap();
    both.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![held_island],
        },
    )
    .unwrap();
    both.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![held_bears],
        },
    )
    .unwrap();

    assert_eq!(
        both.state().players[0].mana_pool.total(),
        4,
        "choosing the alternative cost spends none of the floating mana"
    );
    pass_until(&mut both, stack_is_empty);
    assert!(on_battlefield(&both, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&both, p1, index::GRIZZLY_BEARS).is_some());
}

/// The alternative cost is NOT offered when:
/// - Hand = Foil + Island only (one card short; Foil itself is on the stack
///   while its cost is paid per CR 601.2a and cannot be the second card);
/// - Hand = Foil + two non-Island cards;
/// - Hand = Foil + Island Sanctuary + Grizzly Bears ("Island Sanctuary" has
///   "Island" in its name, not the Island land subtype, CR 205.3i / 305.6).
#[test]
fn foil_withholds_alternative_cost_when_one_card_short_or_without_an_island_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    for (seed, p0_hand, label) in [
        (SEED + 10, &[foil()][..], "Foil alone in hand"),
        (
            SEED + 11,
            &[foil(), island()][..],
            "Foil + Island only (one card short)",
        ),
        (
            SEED + 12,
            &[foil(), index::GRIZZLY_BEARS, index::SAVANNAH_LIONS][..],
            "Foil + two non-Island cards",
        ),
        (
            SEED + 13,
            &[foil(), index::ISLAND_SANCTUARY, index::GRIZZLY_BEARS][..],
            "Foil + Island Sanctuary (name only, not subtype Island) + Bears",
        ),
    ] {
        let mut engine = Duel::new(seed, forest())
            .hand(0, p0_hand)
            .battlefield(1, &[forest(), forest()])
            .hand(1, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut engine);
        reach_their_main_phase(&mut engine, p1);

        cast_from_hand(&mut engine, p1, index::GRIZZLY_BEARS);
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
        );

        let foil_card = in_hand(&engine, p0, foil()).expect("Foil in hand");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        assert!(
            !legal.castable.contains(&foil_card) && !legal.payable.contains(&foil_card),
            "alternative cost must not be offered ({label}): {legal:?}"
        );
        assert!(
            engine
                .apply(p0, PlayerAction::CastSpell { card: foil_card })
                .is_err(),
            "`CastSpell` must refuse Foil with no mana ({label})"
        );
    }

    // Even when `{2}{U}{U}` IS floating, Hand = Foil + Island only does not
    // offer `CastModeKind::Alternative(0)` — the wizard skips `ChooseCastMode`
    // straight to `ChooseTargets` for the normal cost.
    let mut normal_only = Duel::new(SEED + 14, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[foil(), island()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut normal_only);
    reach_their_main_phase(&mut normal_only, p1);
    cast_from_hand(&mut normal_only, p1, index::GRIZZLY_BEARS);
    pass_until(
        &mut normal_only,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut normal_only, p0);
    cast_with_floating(&mut normal_only, p0, foil());
    assert!(
        matches!(normal_only.pending(), Pending::ChooseTargets { .. }),
        "with one card short, only the normal cost is legal so ChooseCastMode is skipped: {:?}",
        normal_only.pending()
    );
}

/// CR 205.3i / 305.6: "an Island card" means any card with the land subtype
/// Island — nonbasics such as Volcanic Island and Tropical Island qualify,
/// Island Sanctuary may pay only the second "another card" slot, and two Island
/// cards in hand may pay both slots as two distinct cards.
#[test]
fn foil_accepts_nonbasic_island_cards_and_two_distinct_islands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // 1. Hand = Foil + Volcanic Island (`Land — Island Mountain`) + Island Sanctuary.
    let mut engine = Duel::new(SEED + 20, forest())
        .hand(
            0,
            &[foil(), index::VOLCANIC_ISLAND, index::ISLAND_SANCTUARY],
        )
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let bolt_spell = on_stack(&engine, lightning_bolt()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let foil_card = in_hand(&engine, p0, foil()).unwrap();
    let volcanic = in_hand(&engine, p0, index::VOLCANIC_ISLAND).unwrap();
    let sanctuary = in_hand(&engine, p0, index::ISLAND_SANCTUARY).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: foil_card })
        .expect("Volcanic Island satisfies 'an Island card'");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bolt_spell],
                players: vec![],
            },
        )
        .unwrap();

    // First discard offers ONLY Volcanic Island, never Island Sanctuary.
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected first discard choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![volcanic]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![volcanic],
            },
        )
        .unwrap();

    // Second discard offers Island Sanctuary as "another card".
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected second discard choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![sanctuary]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sanctuary],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "Lightning Bolt countered"
    );
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, p0, index::VOLCANIC_ISLAND).is_some());
    assert!(in_graveyard(&engine, p0, index::ISLAND_SANCTUARY).is_some());

    // 2. Hand = Foil + Island + Tropical Island: both Island cards are offered
    // for the first discard, and the unchosen one pays "another card".
    let mut two_islands = Duel::new(SEED + 21, forest())
        .hand(0, &[foil(), island(), index::TROPICAL_ISLAND])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut two_islands);
    reach_their_main_phase(&mut two_islands, p1);
    cast_from_hand(&mut two_islands, p1, index::GRIZZLY_BEARS);
    let bears_spell = on_stack(&mut two_islands, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut two_islands,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let foil_card = in_hand(&two_islands, p0, foil()).unwrap();
    let basic_island = in_hand(&two_islands, p0, island()).unwrap();
    let tropical = in_hand(&two_islands, p0, index::TROPICAL_ISLAND).unwrap();

    two_islands
        .apply(p0, PlayerAction::CastSpell { card: foil_card })
        .unwrap();
    two_islands
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_spell],
                players: vec![],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = two_islands.pending().clone() else {
        panic!("expected first discard choice");
    };
    assert_eq!(options, vec![basic_island, tropical]);
    two_islands
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tropical],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = two_islands.pending().clone() else {
        panic!("expected second discard choice");
    };
    assert_eq!(options, vec![basic_island]);
    two_islands
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![basic_island],
            },
        )
        .unwrap();
    pass_until(&mut two_islands, stack_is_empty);
    assert!(in_graveyard(&two_islands, p0, index::TROPICAL_ISLAND).is_some());
    assert!(in_graveyard(&two_islands, p0, island()).is_some());
    assert!(in_graveyard(&two_islands, p1, index::GRIZZLY_BEARS).is_some());
}

/// Normal `{2}{U}{U}` cost + targeting boundaries ("target spell"):
/// - not castable on an empty stack or when only an activated ability is on the
///   stack (CR 113.9);
/// - choosing `CastModeKind::Normal` spends `{2}{U}{U}`, leaves every other card
///   in hand untouched, refuses abilities on the stack as targets, and counters
///   target noncreature spell.
#[test]
fn foil_paid_with_mana_discards_nothing_and_targets_only_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED + 30, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[foil(), island(), index::GRIZZLY_BEARS])
        .battlefield(
            1,
            &[
                mountain(),
                index::PRODIGAL_SORCERER,
                index::PRODIGAL_SORCERER,
            ],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    // Advance through P1's turn to P0's next main phase so P1's Prodigal
    // Sorcerers are no longer summoning-sick.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));

    let hand_before = hand_len(&engine, p0);
    let foil_card = in_hand(&engine, p0, foil()).unwrap();
    let sorcerers = all_on_battlefield(&engine, p1, index::PRODIGAL_SORCERER);
    assert_eq!(sorcerers.len(), 2);

    // Empty stack: Foil has no legal target, so it is not offered even with both
    // `{2}{U}{U}` and the discard alternative available.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.castable.contains(&foil_card) && !legal.payable.contains(&foil_card),
        "Foil is not castable on an empty stack"
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // P1 activates the first Prodigal Sorcerer alone: with ONLY an activated
    // ability on the stack, Foil still has no legal spell target.
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: sorcerers[0],
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected P0 priority");
    };
    assert!(
        !legal.castable.contains(&foil_card) && !legal.payable.contains(&foil_card),
        "an activated ability on the stack is not a spell (CR 113.9)"
    );

    // Let the first ping resolve (P0 -> 19 life); P0 gets priority again in main.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state().players[0].life, 19);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // Now P1 casts Lightning Bolt AND activates the second Prodigal Sorcerer on
    // top of it so a spell and an activated ability coexist on the stack.
    let p1_mountain = on_battlefield(&engine, p1, mountain()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: p1_mountain,
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let bolt_spell = on_stack(&engine, lightning_bolt()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: sorcerers[1],
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let ability_on_stack = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    assert_ne!(ability_on_stack, bolt_spell);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // P0 still has `{U}{U}{U}{U}` floating in the same main phase.
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    cast_with_floating(&mut engine, p0, foil());

    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let normal = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Normal))
        .expect("normal {2}{U}{U} mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert_eq!(options, vec![bolt_spell]);
    assert!(!options.contains(&ability_on_stack));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ability_on_stack],
                    players: vec![],
                },
            )
            .is_err(),
        "Foil refuses targeting an activated ability on the stack"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bolt_spell],
                players: vec![],
            },
        )
        .unwrap();

    // Normal mode asks no discard question, spends all 4 mana, and keeps every
    // other card in P0's hand.
    assert!(on_stack(&engine, foil()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(hand_len(&engine, p0), hand_before - 1);
    assert!(in_hand(&engine, p0, index::GRIZZLY_BEARS).is_some());

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        18,
        "second Sorcerer ping dealt 1, Lightning Bolt's 3 was countered"
    );
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, p0, foil()).is_some());
    assert_eq!(
        hand_len(&engine, p0),
        hand_before - 1,
        "nothing was discarded"
    );
}

/// Stack & rules interactions:
/// 1. Countering Foil after its alternative cost was paid never refunds the two
///    discarded cards (CR 701.6b), and the targeted spell resolves.
/// 2. If Foil's target leaves the stack before Foil resolves, Foil does not
///    resolve and goes to its owner's graveyard (CR 608.2b).
/// 3. Foil may legally target an uncounterable spell (`Abrupt Decay`), resolves
///    without countering it, and Abrupt Decay still resolves.
/// 4. Discarding a card with madness (`Basking Rootwalla`) to pay Foil's
///    alternative cost exiles it and triggers madness (CR 701.9a, 702.35a, 601.2i).
#[test]
fn foil_rules_interactions_countered_fizzled_uncounterable_and_madness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // 1. CR 701.6b: Foil cast via alternative cost is countered by Counterspell ->
    // discarded cards stay in P0's graveyard, and P1's Savannah Lions enters play.
    let mut countered = Duel::new(SEED + 40, island())
        .hand(0, &[foil(), island(), index::GRIZZLY_BEARS])
        .battlefield(1, &[plains(), island(), island()])
        .hand(1, &[index::SAVANNAH_LIONS, index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut countered);
    reach_their_main_phase(&mut countered, p1);

    let p1_plains = on_battlefield(&countered, p1, plains()).unwrap();
    countered
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_plains })
        .unwrap();
    cast_with_floating(&mut countered, p1, index::SAVANNAH_LIONS);
    let lions_spell = on_stack(&countered, index::SAVANNAH_LIONS).unwrap();
    countered.apply(p1, PlayerAction::PassPriority).unwrap();

    let foil_card = in_hand(&countered, p0, foil()).unwrap();
    let p0_island = in_hand(&countered, p0, island()).unwrap();
    let p0_bears = in_hand(&countered, p0, index::GRIZZLY_BEARS).unwrap();
    countered
        .apply(p0, PlayerAction::CastSpell { card: foil_card })
        .unwrap();
    countered
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![lions_spell],
                players: vec![],
            },
        )
        .unwrap();
    countered
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_island],
            },
        )
        .unwrap();
    countered
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_bears],
            },
        )
        .unwrap();
    let foil_on_stack = on_stack(&countered, foil()).unwrap();
    countered.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut countered, p1);
    cast_with_floating(&mut countered, p1, index::COUNTERSPELL);
    countered
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![foil_on_stack],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut countered, stack_is_empty);

    assert_eq!(hand_len(&countered, p0), 0, "no refund of discarded cards");
    assert!(in_graveyard(&countered, p0, island()).is_some());
    assert!(in_graveyard(&countered, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&countered, p0, foil()).is_some());
    assert!(on_battlefield(&countered, p1, index::SAVANNAH_LIONS).is_some());

    // 2. CR 608.2b: Foil's target is countered first by Counterspell on top of
    // Foil -> Foil has no legal target left when it tries to resolve.
    let mut fizzled = Duel::new(SEED + 41, island())
        .battlefield(0, &[island(), island()])
        .hand(
            0,
            &[foil(), island(), index::GRIZZLY_BEARS, index::COUNTERSPELL],
        )
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut fizzled);
    reach_their_main_phase(&mut fizzled, p1);

    cast_from_hand(&mut fizzled, p1, index::GRIZZLY_BEARS);
    let bears_spell = on_stack(&fizzled, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut fizzled,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let foil_card = in_hand(&fizzled, p0, foil()).unwrap();
    let p0_island = in_hand(&fizzled, p0, island()).unwrap();
    let p0_bears = in_hand(&fizzled, p0, index::GRIZZLY_BEARS).unwrap();
    fizzled
        .apply(p0, PlayerAction::CastSpell { card: foil_card })
        .unwrap();
    fizzled
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_spell],
                players: vec![],
            },
        )
        .unwrap();
    fizzled
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_island],
            },
        )
        .unwrap();
    fizzled
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_bears],
            },
        )
        .unwrap();

    // P0 retains priority after casting Foil and counters `bears_spell` on top.
    tap_all_mana(&mut fizzled, p0);
    cast_with_floating(&mut fizzled, p0, index::COUNTERSPELL);
    fizzled
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut fizzled, |e| {
        in_graveyard(e, p1, index::GRIZZLY_BEARS).is_some()
    });
    assert!(
        on_stack(&fizzled, foil()).is_some(),
        "Foil is still on the stack with an illegal target"
    );
    pass_until(&mut fizzled, stack_is_empty);
    assert!(in_graveyard(&fizzled, p0, foil()).is_some());

    // 3. Uncounterable target (`Abrupt Decay`): legal target for Foil, survives
    // Foil's resolution, and destroys P0's Savannah Lions.
    let mut uncounterable = Duel::new(SEED + 42, forest())
        .battlefield(0, &[index::SAVANNAH_LIONS])
        .hand(0, &[foil(), island(), index::GRIZZLY_BEARS])
        .battlefield(1, &[swamp(), forest()])
        .hand(1, &[index::ABRUPT_DECAY])
        .start();
    keep_mulligans(&mut uncounterable);
    reach_their_main_phase(&mut uncounterable, p1);

    let lions = on_battlefield(&uncounterable, p0, index::SAVANNAH_LIONS).unwrap();
    tap_all_mana(&mut uncounterable, p1);
    cast_with_floating(&mut uncounterable, p1, index::ABRUPT_DECAY);
    uncounterable
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![lions],
                players: vec![],
            },
        )
        .unwrap();
    let decay_spell = on_stack(&uncounterable, index::ABRUPT_DECAY).unwrap();
    uncounterable.apply(p1, PlayerAction::PassPriority).unwrap();

    let foil_card = in_hand(&uncounterable, p0, foil()).unwrap();
    let p0_island = in_hand(&uncounterable, p0, island()).unwrap();
    let p0_bears = in_hand(&uncounterable, p0, index::GRIZZLY_BEARS).unwrap();
    uncounterable
        .apply(p0, PlayerAction::CastSpell { card: foil_card })
        .unwrap();
    uncounterable
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![decay_spell],
                players: vec![],
            },
        )
        .unwrap();
    uncounterable
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_island],
            },
        )
        .unwrap();
    uncounterable
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_bears],
            },
        )
        .unwrap();
    pass_until(&mut uncounterable, stack_is_empty);

    assert!(in_graveyard(&uncounterable, p0, foil()).is_some());
    assert!(
        on_battlefield(&uncounterable, p0, index::SAVANNAH_LIONS).is_none()
            && in_graveyard(&uncounterable, p0, index::SAVANNAH_LIONS).is_some(),
        "Abrupt Decay cannot be countered by Foil and destroys Savannah Lions"
    );

    // 4. Madness interaction (CR 701.9a, 702.35a, 601.2i): discarding Basking
    // Rootwalla as Foil's second card exiles it with madness and casts it for {0}.
    let mut madness_duel = Duel::new(SEED + 43, forest())
        .hand(0, &[foil(), island(), index::BASKING_ROOTWALLA])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut madness_duel);
    reach_their_main_phase(&mut madness_duel, p1);

    cast_from_hand(&mut madness_duel, p1, index::GRIZZLY_BEARS);
    let bears_spell = on_stack(&madness_duel, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut madness_duel,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let foil_card = in_hand(&madness_duel, p0, foil()).unwrap();
    let p0_island = in_hand(&madness_duel, p0, island()).unwrap();
    let rootwalla = in_hand(&madness_duel, p0, index::BASKING_ROOTWALLA).unwrap();
    madness_duel
        .apply(p0, PlayerAction::CastSpell { card: foil_card })
        .unwrap();
    madness_duel
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_spell],
                players: vec![],
            },
        )
        .unwrap();
    madness_duel
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_island],
            },
        )
        .unwrap();
    madness_duel
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rootwalla],
            },
        )
        .unwrap();

    // Madness trigger resolves on top of Foil and offers casting Rootwalla for {0}.
    pass_until(&mut madness_duel, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Madness { .. },
                ..
            }
        )
    });
    madness_duel.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut madness_duel, stack_is_empty);

    assert!(
        on_battlefield(&madness_duel, p0, index::BASKING_ROOTWALLA).is_some(),
        "discarded Basking Rootwalla entered via madness"
    );
    assert!(on_battlefield(&madness_duel, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&madness_duel, p1, index::GRIZZLY_BEARS).is_some());
}
