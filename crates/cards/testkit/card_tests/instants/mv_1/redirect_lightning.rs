//! `cards/instants/mv_1/redirect_lightning.rs`, played.
//!
//! Redirect Lightning ({R}) — Instant — Lesson:
//! "As an additional cost to cast this spell, pay 5 life or pay {2}.
//! Change the target of target spell or ability with a single target."

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c62b5c22-e058-436f-9575-a01cb2112829"
fn redirect_lightning() -> CardIndex {
    card_index("c62b5c22-e058-436f-9575-a01cb2112829")
}

fn setup_against_shock(
    p0_life: i32,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, Vec<ObjectId>) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[redirect_lightning()])
        .life(0, p0_life)
        .battlefield(1, &[mountain()])
        .hand(1, &[shock()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, shock());
    lk::aim(&mut engine, &[], &[P0]);
    let shock_spell = on_stack(&engine, shock()).expect("opponent's Shock on stack");
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    let redirect = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(P0))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == redirect_lightning())
        })
        .expect("Redirect Lightning in P0's hand");
    let p0_mountains: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == P0 && o.card.is_some_and(|c| c.index == mountain())
            })
        })
        .collect();
    assert_eq!(p0_mountains.len(), 3, "P0 starts with three Mountains");
    (engine, shock_spell, redirect, p0_mountains)
}

fn tapped_count(engine: &Engine<RegistryLookup>, ids: &[ObjectId]) -> usize {
    ids.iter().filter(|&&id| is_tapped(engine, id)).count()
}

/// CR 601.2b, 118.8, 601.2f, 601.2g: at 20 life, tapping one Mountain for {R}
/// and casting Redirect Lightning off the pool must still offer the {2} branch
/// via `ChooseFirstCostBranch` when untapped mana sources can pay it. Answering
/// "no" opens a CR 601.2g payment window, pays {2} with two more Mountains,
/// keeps life at 20, and redirects Shock to P1.
#[test]
fn redirect_lightning_offers_pay_two_from_pool_route_and_redirects_shock() {
    let (mut engine, shock_spell, redirect, p0_mountains) = setup_against_shock(20);

    // Tap one Mountain for {R} (the pool route), leaving two untapped.
    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountains[0],
            },
        )
        .expect("tap one Mountain for {R}");
    engine
        .apply(P0, PlayerAction::CastSpell { card: redirect })
        .expect("start casting Redirect Lightning with {R} floating");

    // Target opponent's Shock on the stack.
    lk::aim(&mut engine, &[shock_spell], &[]);

    // Engine must ask whether to pay 5 life (yes) or {2} (no).
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::ChooseFirstCostBranch { index: 0 },
                ..
            } if *player == P0
        ),
        "expected ChooseFirstCostBranch when two untapped Mountains can pay {{2}}, got {:?}",
        engine.pending()
    );

    // Answer "no" -> choose the {2} branch -> CR 601.2g payment window opens.
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("choose to pay {2} instead of 5 life");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "expected CR 601.2g payment window for {{2}}, got {:?}",
        engine.pending()
    );

    // Tap the other two Mountains in the payment window and close it.
    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountains[1],
            },
        )
        .expect("tap second Mountain in payment window");
    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountains[2],
            },
        )
        .expect("tap third Mountain in payment window");
    engine
        .apply(P0, PlayerAction::PassPriority)
        .expect("close payment window and put Redirect Lightning on stack");

    // Resolve Redirect Lightning and redirect Shock to P1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("redirect Shock onto P1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::life(&engine, P0), 20, "P0 paid {{2}}, not 5 life");
    assert_eq!(lk::life(&engine, P1), 18, "redirected Shock hit P1 for 2");
    assert_eq!(
        tapped_count(&engine, &p0_mountains),
        3,
        "all three Mountains paid {{2}}{{R}}"
    );
    assert_eq!(lk::pool_total(&engine, P0), 0);
}

/// Same setup at 20 life: answering "yes" to `ChooseFirstCostBranch` pays 5
/// life, leaves the other two Mountains untapped, and redirects Shock to P1.
#[test]
fn redirect_lightning_pay_five_life_branch_leaves_extra_mountains_untapped() {
    let (mut engine, shock_spell, redirect, p0_mountains) = setup_against_shock(20);

    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountains[0],
            },
        )
        .expect("tap one Mountain for {R}");
    engine
        .apply(P0, PlayerAction::CastSpell { card: redirect })
        .expect("start casting Redirect Lightning with {R} floating");

    lk::aim(&mut engine, &[shock_spell], &[]);

    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::ChooseFirstCostBranch { index: 0 },
                ..
            } if *player == P0
        ),
        "expected ChooseFirstCostBranch prompt, got {:?}",
        engine.pending()
    );

    // Answer "yes" -> pay 5 life; {R} in pool already covers the mana cost.
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("choose to pay 5 life");
    assert!(
        on_stack(&engine, redirect_lightning()).is_some(),
        "Redirect Lightning should be on the stack immediately without a payment window"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("redirect Shock onto P1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::life(&engine, P0), 15, "P0 paid 5 life (20 -> 15)");
    assert_eq!(lk::life(&engine, P1), 18, "redirected Shock hit P1 for 2");
    assert_eq!(
        tapped_count(&engine, &p0_mountains),
        1,
        "only the first Mountain was tapped"
    );
}

/// CR 119.4: at 3 life a player cannot pay 5 life, so Redirect Lightning's
/// effective mana floor is {2}{R}, the life branch is never offered, and the
/// cast completes with three Mountains.
#[test]
fn redirect_lightning_at_three_life_prices_at_two_red_and_skips_life_branch() {
    let (mut engine, shock_spell, redirect, p0_mountains) = setup_against_shock(3);

    // Verify floor cost is {2}{R}: tapping only one Mountain is not enough to
    // make Redirect Lightning castable off the pool.
    let face = &baylee_cards::by_index(redirect_lightning())
        .expect("Redirect Lightning in pool")
        .faces[0];
    assert_eq!(
        baylee_engine::casting::face_base_mana_cost(engine.state(), P0, redirect, face),
        baylee_core::mana!("{2}{R}"),
        "at 3 life the 5-life branch is unpayable (CR 119.4), so base price is {{2}}{{R}}"
    );

    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountains[0],
            },
        )
        .expect("tap one Mountain for {R}");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority after tapping one Mountain");
    };
    assert!(
        !legal.castable.contains(&redirect),
        "at 3 life with only {{R}} floating, Redirect Lightning must not be castable off the pool"
    );
    assert!(
        legal.payable.contains(&redirect),
        "with two more Mountains untapped, Redirect Lightning is payable during the cast"
    );

    // Tap the remaining two Mountains so {2}{R} is floating, then cast.
    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountains[1],
            },
        )
        .expect("tap second Mountain");
    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountains[2],
            },
        )
        .expect("tap third Mountain");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority after tapping three Mountains");
    };
    assert!(
        legal.castable.contains(&redirect),
        "with {{R}}{{R}}{{R}} floating at 3 life, Redirect Lightning is castable"
    );

    engine
        .apply(P0, PlayerAction::CastSpell { card: redirect })
        .expect("cast Redirect Lightning for {2}{R}");
    lk::aim(&mut engine, &[shock_spell], &[]);

    // Life branch must not be offered; Redirect Lightning goes straight to stack.
    assert!(
        !matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::ChooseFirstCostBranch { .. },
                ..
            }
        ),
        "life branch must not be offered at 3 life, got {:?}",
        engine.pending()
    );
    assert!(
        on_stack(&engine, redirect_lightning()).is_some(),
        "Redirect Lightning should be on the stack after choosing target"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("redirect Shock onto P1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::life(&engine, P0), 3, "P0 stayed at 3 life");
    assert_eq!(lk::life(&engine, P1), 18, "redirected Shock hit P1 for 2");
    assert_eq!(tapped_count(&engine, &p0_mountains), 3);
    assert_eq!(lk::pool_total(&engine, P0), 0);
}
