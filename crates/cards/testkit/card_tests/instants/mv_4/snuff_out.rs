//! `cards/instants/mv_4/snuff_out.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "324824cb-f938-401c-b9b5-d8908b431ef0"
fn snuff_out() -> CardIndex {
    card_index("324824cb-f938-401c-b9b5-d8908b431ef0")
}

/// Snuff Out: "Destroy target nonblack creature. It can't be regenerated."
///
/// With `{3}{B}` floating and a Swamp controlled, both the normal cost and the
/// 4-life alternative cost are offered; choosing the normal cost spends `{3}{B}`,
/// leaves life unchanged, allows targeting any nonblack creature (yours or an
/// opponent's), refuses black creatures and noncreatures, and destroys the
/// chosen creature.
#[test]
fn snuff_out_destroys_target_nonblack_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9603, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), index::SAVANNAH_LIONS],
        )
        .battlefield(
            1,
            &[
                index::GRIZZLY_BEARS,
                index::DRUDGE_SKELETONS,
                quiet_artifact(),
            ],
        )
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let lions = on_battlefield(&engine, p0, index::SAVANNAH_LIONS).expect("P0 Savannah Lions");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    let skel = on_battlefield(&engine, p1, index::DRUDGE_SKELETONS).expect("P1 Drudge Skeletons");
    let artifact = on_battlefield(&engine, p1, quiet_artifact()).expect("P1 artifact");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, snuff_out());

    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "with {{3}}{{B}} floating and a Swamp controlled, both modes are offered: {:?}",
            engine.pending()
        );
    };
    assert!(
        options
            .iter()
            .any(|opt| matches!(opt.kind, CastModeKind::Alternative(0))),
        "the 4-life alternative is offered beside the normal cost: {options:?}"
    );
    let normal = options
        .iter()
        .position(|opt| matches!(opt.kind, CastModeKind::Normal))
        .expect("the printed {3}{B} mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&bears) && options.contains(&lions),
        "any nonblack creature on the battlefield is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&skel) && !options.contains(&artifact),
        "black creatures and noncreature permanents are excluded: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![skel],
                    players: vec![],
                },
            )
            .is_err(),
        "Snuff Out cannot target a black creature"
    );
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

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().players[0].life, 20);
    assert!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none()
            && in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some(),
        "Snuff Out destroys target nonblack creature into its owner's graveyard"
    );
    assert_eq!(
        on_battlefield(&engine, p0, index::SAVANNAH_LIONS),
        Some(lions)
    );
    assert_eq!(
        on_battlefield(&engine, p1, index::DRUDGE_SKELETONS),
        Some(skel)
    );
}

/// "If you control a Swamp, you may pay 4 life rather than pay this spell's
/// mana cost."
///
/// Played both with zero mana available anywhere (one tapped Swamp, empty pool)
/// and by explicitly selecting `CastModeKind::Alternative(0)` when `{3}{B}` is
/// also floating.
#[test]
fn snuff_out_casts_for_four_life_under_a_swamp_paying_no_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // 1. Caster controls a single Swamp that was tapped on their own turn, so
    // on P1's turn P0 has zero floating mana and no untapped mana sources.
    let mut engine = Duel::new(9604, forest())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    tap_all_mana(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no mana floating"
    );
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    let spell = in_hand(&engine, p0, snuff_out()).expect("Snuff Out in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.mana_abilities.is_empty() && legal.abilities.is_empty(),
        "no mana sources available"
    );
    assert!(
        legal.castable.contains(&spell),
        "controlling a Swamp makes Snuff Out castable for 4 life with no mana: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("castable through the 4-life alternative cost");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].life,
        16,
        "4 life is paid as the spell is cast (CR 601.2h)"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 16);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, snuff_out()).is_some());

    // 2. With `{3}{B}` floating, explicitly choosing the alternative cost in
    // `ChooseCastMode` pays 4 life and leaves all 4 mana in the pool.
    let mut both = Duel::new(9605, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut both);
    assert!(walk_to_own_main(&mut both, p0));
    let bears = on_battlefield(&both, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    tap_all_mana(&mut both, p0);
    cast_with_floating(&mut both, p0, snuff_out());
    let Pending::ChooseCastMode { options, .. } = both.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", both.pending());
    };
    let alt = options
        .iter()
        .position(|opt| matches!(opt.kind, CastModeKind::Alternative(0)))
        .expect("4-life alternative mode offered");
    both.apply(p0, PlayerAction::ChooseMode(alt)).unwrap();
    both.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bears],
            players: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        both.state().players[0].mana_pool.total(),
        4,
        "choosing the 4-life alternative spends none of the floating mana"
    );
    assert_eq!(both.state().players[0].life, 16);
    pass_until(&mut both, stack_is_empty);
    assert!(in_graveyard(&both, p1, index::GRIZZLY_BEARS).is_some());
}

/// Without a Swamp under the caster's own control — whether the board has only
/// non-Swamp lands or only the opponent controls a Swamp — the 4-life
/// alternative cost is not offered, while `{3}{B}` from non-Swamp lands still
/// casts Snuff Out normally.
#[test]
fn snuff_out_withholds_alternative_cost_without_your_own_swamp_and_casts_for_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    for (seed, p1_board, label) in [
        (9606, &[index::GRIZZLY_BEARS][..], "only non-Swamp lands"),
        (
            9607,
            &[swamp(), index::GRIZZLY_BEARS][..],
            "only the opponent controls a Swamp",
        ),
    ] {
        let mut engine = Duel::new(seed, island())
            .battlefield(0, &[island(), island(), island(), index::DROWNED_CATACOMB])
            .battlefield(1, p1_board)
            .hand(0, &[snuff_out()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0));

        let spell = in_hand(&engine, p0, snuff_out()).expect("Snuff Out in hand");
        let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
        let catacomb =
            on_battlefield(&engine, p0, index::DROWNED_CATACOMB).expect("Drowned Catacomb");

        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        assert!(
            !legal.castable.contains(&spell),
            "with an empty pool and no controlled Swamp ({label}), Snuff Out is not castable: {:?}",
            legal.castable
        );

        // Tap only the three Islands (intrinsic CR 305.6 mana abilities) for
        // `{U}{U}{U}` (still no `{B}`): still not castable.
        for source in legal.mana_abilities {
            engine
                .apply(p0, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        assert!(
            !legal.castable.contains(&spell),
            "three blue mana and no controlled Swamp ({label}) still cannot cast Snuff Out"
        );

        // Produce `{B}` from Drowned Catacomb (a non-Swamp land) to reach `{3}{B}`.
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: catacomb,
                    ability_index: 0,
                },
            )
            .unwrap();
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
            .unwrap();

        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        assert!(
            legal.castable.contains(&spell),
            "with {{3}}{{B}} floating ({label}), the normal cost is offered"
        );

        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .unwrap();
        assert!(
            matches!(engine.pending(), Pending::ChooseTargets { .. }),
            "only the normal cost is legal ({label}), so no alternative cast-mode prompt appears: {:?}",
            engine.pending()
        );
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

        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "the normal {{3}}{{B}} cost was spent ({label})"
        );
        assert_eq!(
            engine.state().players[0].life,
            20,
            "no life was paid ({label})"
        );
        assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    }
}

/// "It can't be regenerated" (CR 701.19c): activating River Boa's regeneration
/// ability sets up a real regeneration shield, and Snuff Out still destroys it
/// into its owner's graveyard.
#[test]
fn snuff_out_destroys_through_an_active_regeneration_shield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9608, forest())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[forest(), index::RIVER_BOA])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let boa = on_battlefield(&engine, p1, index::RIVER_BOA).expect("P1 River Boa");
    let spell = in_hand(&engine, p0, snuff_out()).expect("Snuff Out in hand");

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("castable for 4 life under a Swamp");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![boa],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // P1 responds by paying `{G}` to activate River Boa's regenerate ability
    // and letting that ability resolve while Snuff Out waits underneath.
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: boa,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(boa)
            .is_some_and(|obj| obj.regeneration_shields == 1)
    });
    assert!(on_battlefield(&engine, p1, index::RIVER_BOA).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, index::RIVER_BOA).is_none()
            && in_graveyard(&engine, p1, index::RIVER_BOA).is_some(),
        "Snuff Out ignores the active regeneration shield and destroys River Boa"
    );
}

/// CR 119.4: paying 4 life requires a life total of at least 4. At 3 life the
/// 4-life alternative cost is not offered; at 4 life it is.
#[test]
fn snuff_out_respects_the_four_life_payment_boundary() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // At 3 life with a Swamp and no mana, the 4-life alternative is refused.
    let mut at_three = Duel::new(9609, swamp())
        .life(0, 3)
        .battlefield(0, &[swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut at_three);
    assert!(walk_to_own_main(&mut at_three, p0));
    let spell = in_hand(&at_three, p0, snuff_out()).expect("Snuff Out in hand");
    let Pending::Priority { legal, .. } = at_three.pending().clone() else {
        panic!("expected priority, got {:?}", at_three.pending());
    };
    assert!(
        !legal.castable.contains(&spell),
        "at 3 life the 4-life alternative is not offered (CR 119.4): {:?}",
        legal.castable
    );
    tap_all_mana(&mut at_three, p0);
    assert!(
        at_three
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "and `CastSpell` refuses it when `{{3}}{{B}}` is also unavailable"
    );

    // At 3 life with 4 Swamps tapped (`{B}{B}{B}{B}` floating), only the normal
    // cost is offered by the cast wizard — never the 4-life alternative.
    let mut three_with_mana = Duel::new(9610, swamp())
        .life(0, 3)
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut three_with_mana);
    assert!(walk_to_own_main(&mut three_with_mana, p0));
    let bears = on_battlefield(&three_with_mana, p1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut three_with_mana, p0);
    cast_with_floating(&mut three_with_mana, p0, snuff_out());
    assert!(
        matches!(three_with_mana.pending(), Pending::ChooseTargets { .. }),
        "at 3 life only the normal cost is legal, so ChooseCastMode is skipped: {:?}",
        three_with_mana.pending()
    );
    three_with_mana
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut three_with_mana, stack_is_empty);
    assert_eq!(three_with_mana.state().players[0].life, 3);
    assert!(in_graveyard(&three_with_mana, p1, index::GRIZZLY_BEARS).is_some());

    // At 4 life with 1 Swamp and an empty pool, the 4-life alternative IS offered.
    let mut at_four = Duel::new(9611, swamp())
        .life(0, 4)
        .battlefield(0, &[swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut at_four);
    assert!(walk_to_own_main(&mut at_four, p0));
    let spell = in_hand(&at_four, p0, snuff_out()).expect("Snuff Out in hand");
    let Pending::Priority { legal, .. } = at_four.pending().clone() else {
        panic!("expected priority, got {:?}", at_four.pending());
    };
    assert!(
        legal.castable.contains(&spell),
        "at 4 life the 4-life alternative is offered (CR 119.4)"
    );
}

/// CR 701.6b: countering Snuff Out after it was cast for its 4-life alternative
/// cost does not refund the 4 life paid; the target survives and Snuff Out goes
/// to its owner's graveyard.
#[test]
fn snuff_out_countered_after_paying_four_life_does_not_refund_the_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9612, island())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[island(), island(), index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .hand(1, &[index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    let spell = in_hand(&engine, p0, snuff_out()).expect("Snuff Out in hand");

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("castable for 4 life under a Swamp");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].life,
        16,
        "4 life is already paid when Snuff Out goes on the stack"
    );
    let snuff_on_stack = on_stack(&engine, snuff_out()).expect("Snuff Out is on the stack");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // P1 counters Snuff Out with Counterspell.
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, index::COUNTERSPELL);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![snuff_on_stack],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        16,
        "a countered spell never refunds its paid life cost (CR 701.6b)"
    );
    assert_eq!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS),
        Some(bears),
        "the targeted creature survives"
    );
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(
        in_graveyard(&engine, p0, snuff_out()).is_some(),
        "countered Snuff Out is put into P0's graveyard"
    );
}

/// "If you control a Swamp" checks the Swamp land subtype, not the card name or
/// the Basic supertype: a single Underground Sea (`Land — Island Swamp`) as the
/// caster's only land offers and pays the 4-life alternative cost without being
/// tapped.
#[test]
fn snuff_out_accepts_a_nonbasic_swamp_for_its_alternative_cost() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9613, forest())
        .battlefield(0, &[index::UNDERGROUND_SEA])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let sea = on_battlefield(&engine, p0, index::UNDERGROUND_SEA).expect("P0 Underground Sea");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    let spell = in_hand(&engine, p0, snuff_out()).expect("Snuff Out in hand");

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&spell),
        "Underground Sea has the Swamp subtype, so the 4-life alternative is offered: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("castable for 4 life off Underground Sea");
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

    assert_eq!(engine.state().players[0].life, 16);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(
        !is_tapped(&engine, sea),
        "Underground Sea stayed untapped; 4 life paid the cost"
    );
    assert!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none()
            && in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some()
    );
}

/// CR 608.2b: if Snuff Out's sole target becomes illegal before Snuff Out
/// resolves — either because it becomes black in response (Deathlace) or
/// because it leaves the battlefield in response (Unsummon) — Snuff Out does
/// not resolve, goes to its owner's graveyard, and destroys nothing else.
#[test]
fn snuff_out_fizzles_when_its_target_becomes_illegal_before_resolution() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // 1. Target becomes black in response via Deathlace.
    let mut turned_black = Duel::new(9614, swamp())
        .battlefield(0, &[swamp(), index::SAVANNAH_LIONS])
        .battlefield(1, &[swamp(), index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .hand(1, &[index::DEATHLACE])
        .start();
    keep_mulligans(&mut turned_black);
    assert!(walk_to_own_main(&mut turned_black, p0));

    let lions = on_battlefield(&turned_black, p0, index::SAVANNAH_LIONS).unwrap();
    let bears = on_battlefield(&turned_black, p1, index::GRIZZLY_BEARS).unwrap();
    let spell = in_hand(&turned_black, p0, snuff_out()).unwrap();

    turned_black
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    turned_black
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    turned_black.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut turned_black, p1);
    cast_with_floating(&mut turned_black, p1, index::DEATHLACE);
    turned_black
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut turned_black, |e| {
        e.state()
            .object(bears)
            .is_some_and(|obj| obj.characteristics().colors.contains(Color::Black))
    });
    assert!(
        on_stack(&turned_black, snuff_out()).is_some(),
        "Snuff Out is still waiting to resolve after Deathlace made its target black"
    );

    pass_until(&mut turned_black, stack_is_empty);
    assert_eq!(
        on_battlefield(&turned_black, p1, index::GRIZZLY_BEARS),
        Some(bears),
        "now-black target is illegal on resolution and survives (CR 608.2b)"
    );
    assert_eq!(
        on_battlefield(&turned_black, p0, index::SAVANNAH_LIONS),
        Some(lions),
        "bystander creature is untouched"
    );
    assert!(in_graveyard(&turned_black, p0, snuff_out()).is_some());

    // 2. Target leaves the battlefield in response via Unsummon.
    let mut bounced = Duel::new(9615, island())
        .battlefield(0, &[swamp(), index::SAVANNAH_LIONS])
        .battlefield(1, &[island(), index::GRIZZLY_BEARS])
        .hand(0, &[snuff_out()])
        .hand(1, &[index::UNSUMMON])
        .start();
    keep_mulligans(&mut bounced);
    assert!(walk_to_own_main(&mut bounced, p0));

    let lions = on_battlefield(&bounced, p0, index::SAVANNAH_LIONS).unwrap();
    let bears = on_battlefield(&bounced, p1, index::GRIZZLY_BEARS).unwrap();
    let spell = in_hand(&bounced, p0, snuff_out()).unwrap();

    bounced
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    bounced
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    bounced.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut bounced, p1);
    cast_with_floating(&mut bounced, p1, index::UNSUMMON);
    bounced
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut bounced, stack_is_empty);

    assert!(
        in_hand(&bounced, p1, index::GRIZZLY_BEARS).is_some()
            && in_graveyard(&bounced, p1, index::GRIZZLY_BEARS).is_none(),
        "bounced target is in P1's hand, not destroyed"
    );
    assert_eq!(
        on_battlefield(&bounced, p0, index::SAVANNAH_LIONS),
        Some(lions),
        "bystander creature is untouched"
    );
    assert!(in_graveyard(&bounced, p0, snuff_out()).is_some());
}

/// CR 702.12b: a nonblack indestructible creature is a legal target for Snuff
/// Out, and resolving Snuff Out against it does not destroy it ("can't be
/// regenerated" suppresses regeneration shields, not indestructible).
#[test]
fn snuff_out_targets_a_nonblack_indestructible_creature_without_destroying_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9616, swamp())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[index::DARKSTEEL_GARGOYLE])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let gargoyle =
        on_battlefield(&engine, p1, index::DARKSTEEL_GARGOYLE).expect("P1 Darksteel Gargoyle");
    let spell = in_hand(&engine, p0, snuff_out()).expect("Snuff Out in hand");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&spell),
        "a colorless indestructible creature is a legal target, so Snuff Out is castable"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&gargoyle), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gargoyle],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 16);
    assert_eq!(
        on_battlefield(&engine, p1, index::DARKSTEEL_GARGOYLE),
        Some(gargoyle),
        "Darksteel Gargoyle has indestructible and stays on the battlefield (CR 702.12b)"
    );
    assert!(in_graveyard(&engine, p1, index::DARKSTEEL_GARGOYLE).is_none());
    assert!(in_graveyard(&engine, p0, snuff_out()).is_some());
}
