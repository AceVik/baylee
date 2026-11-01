//! `cards/creatures/mv_4/fungus_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn fungus_elemental() -> CardIndex {
    card_index("3d3c5dd5-ad4e-48ca-92b1-91e1f4ad5c3b")
}

/// The Fungus cast and resolved on P0's first turn, off exactly the four
/// green mana its cost asks, on a board of the named lands.
fn fungus_on(lands: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = lk::game(SEED, lands, &[], &[fungus_elemental()], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut e, P0, fungus_elemental());
    pass_until(&mut e, stack_is_empty);
    let fungus = on_battlefield(&e, P0, fungus_elemental()).expect("the Fungus is out");
    assert_eq!(pt(&e, fungus), (3, 3));
    (e, fungus)
}

/// Fungus Elemental (3d3c5dd5-ad4e-48ca-92b1-91e1f4ad5c3b) — "{G}, Sacrifice
/// a Forest: Put a +2/+2 counter on this creature. Activate only if this
/// creature entered this turn." On its first turn: {G} is spent, only a
/// Forest is on the menu (the Mountain is not), the chosen Forest is
/// sacrificed, and the Fungus is a 5/5.
#[test]
fn fungus_elemental_sacrifices_a_forest_on_its_first_turn_for_a_counter() {
    let (mut e, fungus) = fungus_on(&[forest(), forest(), mountain()]);
    let forests = all_on_battlefield(&e, P0, forest());
    let mountain_land = lk::obj(&e, P0, mountain());
    assert_eq!(forests.len(), 2);

    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    assert!(lk::offered(&e, fungus_elemental(), P0, 0));
    activate(&mut e, P0, fungus_elemental(), 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("the cost asks which Forest to sacrifice: {:?}", e.pending())
    };
    assert!(
        forests.iter().all(|f| options.contains(f)),
        "both Forests are offered: {options:?}"
    );
    assert!(
        !options.contains(&mountain_land),
        "a Mountain is not a Forest: {options:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![forests[0]],
        },
    )
    .expect("the Forest is sacrificed");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, fungus), (5, 5), "one +2/+2 counter");
    assert_eq!(lk::pool_total(&e, P0), 0, "the {{G}} was paid");
    assert!(
        e.state()
            .object(forests[0])
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "the Forest is gone"
    );
    assert!(on_battlefield(&e, P0, mountain()).is_some());
    assert_eq!(all_on_battlefield(&e, P0, forest()).len(), 1);
}

/// Fungus Elemental — "Activate only if this creature entered this turn": on
/// a later turn, with a Forest and {G} to spare, the ability is not offered.
#[test]
fn fungus_elemental_cannot_do_it_on_a_later_turn() {
    let (mut e, fungus) = fungus_on(&[forest(), forest()]);
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    assert!(
        !lk::offered(&e, fungus_elemental(), P0, 0),
        "it entered two turns ago"
    );
    assert_eq!(pt(&e, fungus), (3, 3));
}

/// Fungus Elemental — "Sacrifice a Forest": with only a Mountain to give,
/// the ability is not offered even on the turn the Fungus entered.
#[test]
fn fungus_elemental_needs_a_forest_to_sacrifice() {
    let (mut e, _fungus) = fungus_on(&[mountain()]);
    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    assert!(!lk::offered(&e, fungus_elemental(), P0, 0));
}
