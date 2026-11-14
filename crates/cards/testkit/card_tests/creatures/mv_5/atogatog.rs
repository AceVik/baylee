//! `cards/creatures/mv_5/atogatog.rs`, played.
//!
//! Oracle:
//! - "Sacrifice an Atog creature: Atogatog gets +X/+X until end of turn, where X
//!   is the sacrificed creature's power."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn atogatog() -> CardIndex {
    card_index("b9fdb740-e5d7-4464-b378-2e0514ca28d8")
}

fn activate_and_check_menu(
    engine: &mut Engine<RegistryLookup>,
    expected: &[ObjectId],
    excluded: &[ObjectId],
) {
    activate(engine, P0, atogatog(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostSacrifice prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert_eq!((min, max), (1, 1));
    for id in expected {
        assert!(
            options.contains(id),
            "expected Atog {id:?} on sacrifice menu"
        );
    }
    for id in excluded {
        assert!(
            !options.contains(id),
            "non-Atog or opponent Atog {id:?} must not be on menu"
        );
    }
}

#[test]
fn atogatog_sacrifices_atog_for_power_pump_and_clamps_negative_power() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                island(),
                swamp(),
                swamp(),
                mountain(),
                forest(),
                index::ATOG,
                index::ATOG,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::ATOG])
        .hand(0, &[atogatog(), index::WEAKNESS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_atogs = all_on_battlefield(&engine, P0, index::ATOG);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let p1_atog = on_battlefield(&engine, PlayerId::new(1), index::ATOG).unwrap();

    // Cast Atogatog with {W}{U}{B}{R}{G}, leaving the second Swamp for Weakness.
    let reserve_swamp = all_on_battlefield(&engine, P0, swamp())[1];
    for land in lands_of(&engine, P0) {
        if land != reserve_swamp {
            engine
                .apply(P0, PlayerAction::ActivateManaAbility { source: land })
                .unwrap();
        }
    }
    cast_with_floating(&mut engine, P0, atogatog());
    pass_until(&mut engine, stack_is_empty);

    let tog = on_battlefield(&engine, P0, atogatog()).expect("Atogatog resolved");
    assert_eq!(pt(&engine, tog), (5, 5));
    assert!(
        engine
            .state()
            .object(tog)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY)
    );

    // Give the second Atog -2/-1 via Weakness so its power is -1 (CR 107.1b -> X = 0).
    let neg_atog = p0_atogs[1];
    cast_from_hand(&mut engine, P0, index::WEAKNESS);
    aim_at(&mut engine, P0, neg_atog);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, neg_atog), (-1, 1));

    // Menu offers own Atogs (including Atogatog) and excludes Grizzly Bears & opponent's Atog.
    activate_and_check_menu(
        &mut engine,
        &[p0_atogs[0], neg_atog, tog],
        &[bears, p1_atog],
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_atogs[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, tog),
        (6, 6),
        "1/2 Atog pumped Atogatog by +1/+1"
    );

    // Sacrificing a -1/1 Atog yields X = 0 under CR 107.1b, leaving Atogatog at 6/6.
    activate(&mut engine, P0, atogatog(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![neg_atog],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, tog), (6, 6));

    // Pump expires at end of turn.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, tog), (5, 5));
}
