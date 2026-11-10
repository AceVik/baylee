//! `cards/creatures/mv_3/orcish_farmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes::land};

fn orcish_farmer() -> CardIndex {
    card_index("c3039d19-8c98-4953-8943-9922b6ab45ef")
}

/// "{T}: Target land becomes a Swamp" offers lands on either side, never a
/// creature, and pays its tap cost. The changed Forest loses its old basic land
/// type and produces exactly {B} through the ordinary intrinsic mana action.
#[test]
fn orcish_farmer_targets_a_land_and_makes_it_tap_for_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                forest(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[forest()])
        .hand(0, &[orcish_farmer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    cast_with_floating(&mut engine, p0, orcish_farmer());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let farmer = on_battlefield(&engine, p0, orcish_farmer()).unwrap();
    let target = on_battlefield(&engine, p0, forest()).unwrap();
    let opposing_land = on_battlefield(&engine, p1, forest()).unwrap();
    let creature = on_battlefield(&engine, p0, festering_goblin()).unwrap();
    assert!(
        ability_offered(&engine, farmer, 0),
        "the tap ability is available once summoning sickness ends"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: farmer,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the activation asks for a land, got {:?}", engine.pending());
    };
    assert!(options.contains(&target));
    assert!(
        options.contains(&opposing_land),
        "the printed target is any land, not only one you control"
    );
    assert!(!options.contains(&farmer) && !options.contains(&creature));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![creature],
                    players: vec![],
                },
            )
            .is_err(),
        "a creature is not a legal land target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, farmer), "the {{T}} cost is paid");
    pass_until(&mut engine, stack_is_empty);

    let subtypes = engine
        .state()
        .object(target)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subtypes.contains(land::SWAMP));
    assert!(
        !subtypes.contains(land::FOREST),
        "becoming a Swamp replaces the former basic land type"
    );
    assert!(
        priority_offer(&engine).mana_abilities.contains(&target),
        "the projected Swamp offers its intrinsic mana ability"
    );
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: target })
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.total(), 1);
    assert!(
        !ability_offered(&engine, farmer, 0),
        "the tapped Farmer cannot pay {{T}} again"
    );
}

/// Activated during p1's turn, the change survives that turn and p0's intervening
/// turn. At p1's next untap step it ends before untap determination: the land is
/// an Island again, so Choke keeps the already-tapped permanent down (CR 502.3).
#[test]
fn orcish_farmer_expires_before_the_affected_controllers_next_untap_determination() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), index::CHOKE])
        .battlefield(1, &[island()])
        .hand(0, &[orcish_farmer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, orcish_farmer());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    let farmer = on_battlefield(&engine, p0, orcish_farmer()).unwrap();
    let island = on_battlefield(&engine, p1, island()).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(ability_offered(&engine, farmer, 0));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: farmer,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![island],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let changed = engine
        .state()
        .object(island)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(changed.contains(land::SWAMP) && !changed.contains(land::ISLAND));
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: island })
        .expect("the new Swamp taps for {B}");
    assert_eq!(engine.state().players[1].mana_pool.total(), 1);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert!(is_tapped(&engine, island));

    reach_their_main_phase(&mut engine, p0);
    let between_turns = engine
        .state()
        .object(island)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        between_turns.contains(land::SWAMP) && !between_turns.contains(land::ISLAND),
        "only the affected land's controller's next untap step ends the effect"
    );
    assert!(is_tapped(&engine, island));

    reach_their_main_phase(&mut engine, p1);
    let restored = engine
        .state()
        .object(island)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(restored.contains(land::ISLAND) && !restored.contains(land::SWAMP));
    assert!(
        is_tapped(&engine, island),
        "the Island returns before Choke's untap prohibition is evaluated"
    );
}
