//! `cards/creatures/mv_2/orcish_settlers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn orcish_settlers() -> CardIndex {
    card_index("8b8d4ae0-0d18-4f06-a883-0edff494d06b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The Settlers and six Mountains against a Plains, an Island, a Forest and a
/// Grizzly Bears, with all six Mountains' mana floating.
fn table() -> Engine<RegistryLookup> {
    let mut e = lk::game(
        SEED,
        &[
            orcish_settlers(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
        ],
        &[plains(), island(), forest(), grizzly_bears()],
        &[],
        &[],
    );
    tap_all_mana(&mut e, P0);
    assert_eq!(lk::pool_total(&e, P0), 6, "six Mountains' worth of mana");
    e
}

/// Orcish Settlers (8b8d4ae0-0d18-4f06-a883-0edff494d06b) — "{X}{X}{R}, {T},
/// Sacrifice this creature: Destroy X target lands." With X = 2 it costs
/// {2}{2}{R} (5 of the 6 floating), asks for exactly two targets from the
/// lands (the Bears and the Settlers themselves are no land), destroys those
/// two and no third, and the Settlers are sacrificed.
#[test]
fn orcish_settlers_destroys_exactly_x_lands_for_xx_r() {
    let mut e = table();
    let plains_id = lk::obj(&e, P1, plains());
    let island_id = lk::obj(&e, P1, island());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let settlers = lk::obj(&e, P0, orcish_settlers());

    activate(&mut e, P0, orcish_settlers(), 0);
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets {
        min, max, options, ..
    } = e.pending().clone()
    else {
        panic!("X lands are asked for, got {:?}", e.pending())
    };
    assert_eq!((min, max), (2, 2), "exactly X targets");
    assert!(options.contains(&plains_id) && options.contains(&island_id));
    assert!(
        !options.contains(&bears),
        "a creature is no land: {options:?}"
    );
    assert!(
        !options.contains(&settlers),
        "the Settlers are no land: {options:?}"
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![plains_id],
                players: vec![],
            },
        )
        .is_err(),
        "one target is one short of X = 2"
    );
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![plains_id, island_id],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, orcish_settlers()).is_some(),
        "sacrificed as part of the cost"
    );
    assert!(in_graveyard(&e, P1, plains()).is_some());
    assert!(in_graveyard(&e, P1, island()).is_some());
    assert!(
        on_battlefield(&e, P1, forest()).is_some(),
        "the third land is not a target"
    );
    assert!(on_battlefield(&e, P1, grizzly_bears()).is_some());
    assert_eq!(
        lk::pool_total(&e, P0),
        1,
        "{{X}}{{X}}{{R}} with X = 2 is five mana"
    );
}

/// Orcish Settlers — X = 1 costs {1}{1}{R} (3 of the 6) and destroys one
/// land only: the Forest goes, the Plains and the Island stay.
#[test]
fn orcish_settlers_with_x_one_destroys_one_land() {
    let mut e = table();
    let forest_id = lk::obj(&e, P1, forest());

    activate(&mut e, P0, orcish_settlers(), 0);
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![forest_id],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P1, forest()).is_some());
    assert!(on_battlefield(&e, P1, plains()).is_some());
    assert!(on_battlefield(&e, P1, island()).is_some());
    assert!(in_graveyard(&e, P0, orcish_settlers()).is_some());
    assert_eq!(lk::pool_total(&e, P0), 3, "X = 1 is three mana");
}
