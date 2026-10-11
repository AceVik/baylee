//! `cards/creatures/mv_3/renegade_rallier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Revolt returns a cheap permanent card, after one of your permanents left.
/// An instant in the same graveyard is never a legal target.
#[test]
fn renegade_rallier_reanimates_a_cheap_permanent_card_after_revolt() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[forest(), forest(), forest(), plains(), swamp()])
        .hand(0, &[renegade_rallier(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let departed = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            departed,
            ZoneLocation::Hand(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    // Two cards the trigger may take, off the top of a deck of Forests.
    seed_graveyard(&mut engine, p0, 2);

    // And one it may not. Nothing has left the battlefield to pay for this
    // either: an instant resolving is a card changing zones, not a permanent.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let ritual = in_graveyard(&engine, p0, dark_ritual())
        .expect("Dark Ritual resolved and went to its owner's graveyard");

    let forests_before = all_on_battlefield(&engine, p0, forest()).len();
    let board_before = engine.state().zones.list(ZoneLocation::Battlefield).clone();

    cast_from_hand(&mut engine, p0, renegade_rallier());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    // After the earlier departure, resolving Rallier adds just Rallier.
    let now = engine.state().zones.list(ZoneLocation::Battlefield);
    assert!(
        board_before.iter().all(|id| now.contains(id)),
        "no further permanent leaves during the cast"
    );
    assert_eq!(
        now.len(),
        board_before.len() + 1,
        "the Rallier's own arrival is the only thing that has happened"
    );

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Rallier's controller is the one asked");
    assert_eq!((min, max), (1, 1), "exactly one card comes back");
    assert!(
        player_options.is_empty(),
        "\"target permanent card\" reaches no player: {player_options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "the two Forests: lands, mana value 0, and nothing else: {options:?}"
    );
    assert!(
        !options.contains(&ritual),
        "Dark Ritual is an instant, so it is no permanent card (CR 110.4a): {options:?}"
    );

    let taken = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![taken],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let landed = engine
        .state()
        .object(taken)
        .expect("the returned card is still an object");
    assert_eq!(
        landed.zone,
        crate::zone::Zone::Battlefield,
        "\"return target permanent card ... to the battlefield\""
    );
    assert_eq!(
        landed.controller, p0,
        "under the control of the seat that cast it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        forests_before + 1,
        "one Forest came back out of the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_some(),
        "and the instant the menu refused stayed exactly where it was"
    );

    let rallier =
        on_battlefield(&engine, p0, renegade_rallier()).expect("the Rallier is on the table");
    assert_eq!(pt(&engine, rallier), (3, 2), "and it is the printed 3/2");
}

#[test]
fn revolt_requires_your_departure_in_this_turn_and_not_phasing_or_milling() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for case in 0..4 {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[renegade_rallier()])
            .battlefield(0, &[index::GRIZZLY_BEARS])
            .battlefield(1, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        seed_graveyard(&mut e, p0, 2);
        let bear =
            on_battlefield(&e, if case == 1 { p1 } else { p0 }, index::GRIZZLY_BEARS).unwrap();
        if case == 1 || case == 3 {
            bury(&mut e, &[bear]);
        }
        if case == 2 {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(bear)
                .unwrap()
                .status
                .insert(crate::object::Status::PHASED_OUT);
        }
        if case == 3 {
            reach_their_main_phase(&mut e, p1);
            reach_their_main_phase(&mut e, p0);
        }
        let before = e.state().zones.list(ZoneLocation::Graveyard(p0)).len();
        for color in [ManaColor::Green, ManaColor::White] {
            e.dev_state_mut(p0).unwrap().players[0]
                .mana_pool
                .add(color, 2);
        }
        e.refresh_offer();
        cast_with_floating(&mut e, p0, renegade_rallier());
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
            before,
            "case {case}"
        );
        assert!(!e.state().per_turn.revolt[0]);
    }
}
