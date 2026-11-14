//! `cards/2001/src/ody/bearscape.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bearscape() -> CardIndex {
    card_index("7e481e22-4122-4259-8978-4727eb307195")
}

/// Activating Bearscape requires {1}{G} and two cards in YOUR graveyard
/// (opponent's graveyard cards do not count); paying both exiles the two
/// chosen cards to your exile zone and creates a 2/2 green Bear token.
#[test]
fn bearscape_exiles_two_from_your_graveyard_for_a_two_two_bear() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(
            0,
            &[bearscape(), island(), grizzly_bears(), lightning_bolt()],
        )
        .hand(1, &[mountain(), swamp(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, bearscape());
    pass_until(&mut engine, stack_is_empty);
    let scape = on_battlefield(&engine, P0, bearscape()).expect("Bearscape on board");

    // 3 cards in P1's graveyard, only 1 in P0's graveyard + {1}{G} floating.
    let p1_mtn = hand_to_graveyard(&mut engine, P1, mountain());
    let _p1_swp = hand_to_graveyard(&mut engine, P1, swamp());
    let _p1_pln = hand_to_graveyard(&mut engine, P1, plains());
    let my_island = hand_to_graveyard(&mut engine, P0, island());
    tap_all_mana(&mut engine, P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(!legal.abilities.iter().any(|(s, _)| *s == scape));
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: scape,
                    ability_index: 0,
                },
            )
            .is_err()
    );

    // Add 2 more cards to P0's graveyard -> now 3 cards available.
    let my_bears = hand_to_graveyard(&mut engine, P0, grizzly_bears());
    let my_bolt = hand_to_graveyard(&mut engine, P0, lightning_bolt());

    activate(&mut engine, P0, bearscape(), 0);
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected first graveyard exile choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&my_island) && options.contains(&my_bears) && options.contains(&my_bolt)
    );
    assert!(!options.contains(&p1_mtn), "opponent graveyard is excluded");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![my_island],
            },
        )
        .unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected second graveyard exile choice, got {:?}",
            engine.pending()
        );
    };
    assert!(
        !options.contains(&my_island),
        "already chosen card cannot be picked twice"
    );
    assert!(options.contains(&my_bears) && options.contains(&my_bolt));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![my_bears],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let p0_exile = engine.state().zones.list(ZoneLocation::Exile(P0));
    assert!(p0_exile.contains(&my_island) && p0_exile.contains(&my_bears));
    assert!(in_graveyard(&engine, P0, island()).is_none());
    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, P0, lightning_bolt()).is_some());

    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    let chars = engine.state().object(tokens[0]).unwrap().characteristics();
    assert_eq!(chars.colors, ColorSet::of(Color::Green));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.subtypes.contains(subtypes::creature::BEAR));
}
