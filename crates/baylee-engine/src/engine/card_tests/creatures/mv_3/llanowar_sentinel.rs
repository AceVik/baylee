//! `cards/creatures/mv_3/llanowar_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn llanowar_sentinel() -> CardIndex {
    card_index("272ab5d5-7edb-4457-8a09-cabe2325c881")
}

/// How many copies of `card` `seat` has in the given zone.
fn count_in(e: &Engine<RegistryLookup>, zone: ZoneLocation, card: CardIndex) -> usize {
    e.state()
        .zones
        .list(zone)
        .iter()
        .filter(|id| {
            e.state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// Five Forests, a Sentinel to cast and its twin sitting in the library; the
/// cast has resolved and the "may pay {1}{G}" question is up, with exactly
/// the {1}{G} left floating.
fn sentinel_enters_with_a_twin_in_the_library() -> Engine<RegistryLookup> {
    let mut e = lk::game(
        SEED,
        &[forest(), forest(), forest(), forest(), forest()],
        &[],
        &[llanowar_sentinel(), llanowar_sentinel()],
        &[],
    );
    hand_to_library_top(&mut e, P0, llanowar_sentinel());
    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, llanowar_sentinel());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e
}

/// Llanowar Sentinel (272ab5d5-7edb-4457-8a09-cabe2325c881) — "When this
/// creature enters, you may pay {1}{G}. If you do, search your library for a
/// card named Llanowar Sentinel, put that card onto the battlefield, then
/// shuffle." Paid, the search offers the twin and nothing else (the library
/// is Forests), the twin enters, and its own enters-trigger asks again.
#[test]
fn llanowar_sentinel_paid_fetches_its_twin_from_the_library() {
    let mut e = sentinel_enters_with_a_twin_in_the_library();
    assert_eq!(
        count_in(&e, ZoneLocation::Library(P0), llanowar_sentinel()),
        1
    );

    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        options.len(),
        1,
        "only a card named Llanowar Sentinel: {options:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: options.clone(),
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        count_in(&e, ZoneLocation::Battlefield, llanowar_sentinel()),
        2,
        "the cast one and its twin"
    );
    assert_eq!(
        count_in(&e, ZoneLocation::Library(P0), llanowar_sentinel()),
        0
    );
}

/// Llanowar Sentinel — "you may pay": declined, the mana stays in the pool,
/// the twin stays in the library and only one Sentinel is out.
#[test]
fn llanowar_sentinel_declined_fetches_nothing() {
    let mut e = sentinel_enters_with_a_twin_in_the_library();

    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        count_in(&e, ZoneLocation::Battlefield, llanowar_sentinel()),
        1
    );
    assert_eq!(
        count_in(&e, ZoneLocation::Library(P0), llanowar_sentinel()),
        1
    );
    assert_eq!(lk::pool_total(&e, P0), 2, "the {{1}}{{G}} was not spent");
}
