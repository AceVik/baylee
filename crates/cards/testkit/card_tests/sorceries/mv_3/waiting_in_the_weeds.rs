//! `cards/sorceries/mv_3/waiting_in_the_weeds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_cards_dsl::{Color, ColorSet};
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn waiting_in_the_weeds() -> CardIndex {
    card_index("5a61d51e-1a3f-49f6-9a94-a708dd6f69df")
}

/// Waiting in the Weeds (5a61d51e-1a3f-49f6-9a94-a708dd6f69df) — {1}{G}{G}:
/// "Each player creates a 1/1 green Cat creature token for each untapped Forest
/// they control."
#[test]
fn waiting_in_the_weeds_creates_cat_tokens_per_untapped_forest_for_each_player() {
    let mut engine = lk::game(
        SEED,
        &[
            plains(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            island(),
        ],
        &[forest(), forest(), forest(), island(), island()],
        &[waiting_in_the_weeds()],
        &[],
    );
    let my_plains = lk::obj(&engine, P0, plains());
    let my_forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == P0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    let their_forest = lk::obj(&engine, P1, forest());

    // Tap one of P1's three Forests -> P1 has 2 untapped Forests + 2 untapped Islands.
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(their_forest)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    // Tap P0's Plains + 2 of P0's 5 Forests for {W}{G}{G}, leaving P0 with 3 untapped Forests + 1 untapped Island.
    let spend = [my_plains, my_forests[0], my_forests[1]];
    tap_mana_where(&mut engine, P0, |id| spend.contains(&id));
    cast_with_floating(&mut engine, P0, waiting_in_the_weeds());
    lk::settle(&mut engine);

    let p0_cats = tokens_of(&engine, P0);
    let p1_cats = tokens_of(&engine, P1);
    assert_eq!(p0_cats.len(), 3, "P0 had 3 untapped Forests");
    assert_eq!(p1_cats.len(), 2, "P1 had 2 untapped Forests");

    for cat in p0_cats.into_iter().chain(p1_cats) {
        assert_eq!(pt(&engine, cat), (1, 1));
        let ch = engine.state().object(cat).unwrap().characteristics();
        assert!(ch.types.contains(TypeSet::CREATURE));
        assert_eq!(ch.colors, ColorSet::from_slice(&[Color::Green]));
        assert!(ch.subtypes.contains(subtypes::creature::CAT));
    }
}
