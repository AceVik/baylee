//! Jinx's five resolution choices, temporary land type and one next-turn-upkeep draw.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit::{self as lk, P0, P1};
use baylee_core::generated::{index, subtypes::land};
use baylee_core::ids::SubtypeId;

fn jinx() -> CardIndex {
    card_index("e6e1b51c-ba4b-4997-968a-751b4cba110f")
}

fn basic_types() -> [SubtypeId; 5] {
    [
        land::PLAINS,
        land::ISLAND,
        land::SWAMP,
        land::MOUNTAIN,
        land::FOREST,
    ]
}

fn hand_len(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

fn assert_basic_type(engine: &Engine<RegistryLookup>, target: ObjectId, expected: SubtypeId) {
    let chars = engine.state().object(target).unwrap().characteristics();
    for kind in basic_types() {
        assert_eq!(chars.subtypes.contains(kind), kind == expected);
    }
}

fn next_upkeep(engine: &mut Engine<RegistryLookup>, active: PlayerId) {
    pass_until(engine, |e| {
        e.state().turn.active == active
            && e.state().turn.step == crate::turn::Step::Upkeep
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

fn cast_and_choose(choice: usize) -> (Engine<RegistryLookup>, ObjectId, usize) {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), index::GRIZZLY_BEARS],
        &[mountain(), forest()],
        &[jinx()],
        &[],
    );
    let target = on_battlefield(&engine, P1, mountain()).unwrap();
    let own_land = on_battlefield(&engine, P0, island()).unwrap();
    let creature = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let hand_before = hand_len(&engine, P0);
    cast_from_hand(&mut engine, P0, jinx());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("land target");
    };
    assert!(options.contains(&own_land) && options.contains(&target));
    assert!(!options.contains(&creature));
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![creature],
                    players: vec![]
                }
            )
            .is_err()
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "type is chosen on resolution, after the target"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, P0);
    assert_eq!(options.len(), 5);
    engine.apply(P0, PlayerAction::ChooseMode(choice)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let other_land = on_battlefield(&engine, P1, forest()).unwrap();
    assert_basic_type(&engine, other_land, land::FOREST);
    (engine, target, hand_before)
}

#[test]
fn jinx_chooses_each_basic_land_type_on_resolution_then_expires_and_draws_once() {
    let mana_colors = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ];
    for (choice, chosen_type) in basic_types().into_iter().enumerate() {
        let (mut engine, target, hand_before) = cast_and_choose(choice);
        assert_basic_type(&engine, target, chosen_type);
        assert_eq!(hand_len(&engine, P0), hand_before - 1, "no immediate draw");
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
        );
        engine
            .apply(P1, PlayerAction::ActivateManaAbility { source: target })
            .unwrap();
        assert_eq!(lk::pool_of(&engine, P1, mana_colors[choice]), 1);
        assert_eq!(lk::pool_total(&engine, P1), 1);
        pass_until(&mut engine, |e| {
            e.state().turn.active == P0 && e.state().turn.step == crate::turn::Step::End
        });
        assert_eq!(
            hand_len(&engine, P0),
            hand_before - 1,
            "not drawn at the end step"
        );
        next_upkeep(&mut engine, P1);
        assert_eq!(
            hand_len(&engine, P0),
            hand_before,
            "caster draws in the next turn's upkeep"
        );
        assert_eq!(
            hand_len(&engine, P1),
            0,
            "the active player did not receive Jinx's draw"
        );
        assert_basic_type(&engine, target, land::MOUNTAIN);
        next_upkeep(&mut engine, P0);
        assert_eq!(
            hand_len(&engine, P0),
            hand_before,
            "delayed draw is consumed, not repeated on the caster's next upkeep"
        );
    }
}
