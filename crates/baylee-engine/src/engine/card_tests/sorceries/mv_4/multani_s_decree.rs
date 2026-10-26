//! `cards/sorceries/mv_4/multani_s_decree.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn multani_s_decree() -> CardIndex {
    card_index("b233473d-d22b-45e4-9fe7-55d1170a788b")
}

/// Multani's Decree (b233473d-d22b-45e4-9fe7-55d1170a788b) — "Destroy all
/// enchantments. You gain 2 life for each enchantment destroyed this way."
#[test]
fn multanis_decree_destroys_all_enchantments_and_gains_two_life_for_each_destroyed() {
    let mut e = lk::game(
        SEED,
        &[fastbond(), sol_ring()],
        &[mirri_s_guile(), sylvan_library()],
        &[multani_s_decree()],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut e, lk::P0, multani_s_decree());
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P0, fastbond()).is_some());
    assert!(in_graveyard(&e, lk::P1, mirri_s_guile()).is_some());
    assert!(in_graveyard(&e, lk::P1, sylvan_library()).is_some());
    assert!(
        on_battlefield(&e, lk::P0, sol_ring()).is_some(),
        "non-enchantments are untouched"
    );
    assert_eq!(
        lk::life(&e, lk::P0),
        26,
        "3 enchantments destroyed * 2 life = +6 life"
    );
    assert_eq!(lk::life(&e, lk::P1), 20);
}

/// Multani's Decree — CR 701.8c: an indestructible enchantment is not
/// destroyed this way and does not grant 2 life.
#[test]
fn multanis_decree_counts_only_enchantments_actually_destroyed_this_way() {
    let mut e = lk::game(
        SEED,
        &[fastbond()],
        &[mirri_s_guile()],
        &[heroic_intervention(), multani_s_decree()],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 6)]);
    cast_with_floating(&mut e, lk::P0, heroic_intervention());
    lk::settle(&mut e);

    cast_with_floating(&mut e, lk::P0, multani_s_decree());
    lk::settle(&mut e);

    assert!(
        on_battlefield(&e, lk::P0, fastbond()).is_some(),
        "indestructible enchantment survives"
    );
    assert!(in_graveyard(&e, lk::P1, mirri_s_guile()).is_some());
    assert_eq!(
        lk::life(&e, lk::P0),
        22,
        "only the 1 enchantment actually destroyed grants 2 life"
    );
}

#[test]
fn multanis_decree_counts_destroyed_enchantments_exiled_instead_of_dying() {
    let voidwalker = card_index("f1c2dbe2-fbe0-4058-bdf1-91d1b1832786");
    let mut e = lk::game(
        SEED,
        &[fastbond(), sol_ring()],
        &[voidwalker, mirri_s_guile()],
        &[multani_s_decree()],
        &[],
    );
    let journal_from = e.state().journal.entries().len();
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut e, lk::P0, multani_s_decree());
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P0, fastbond()).is_none());
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Exile(lk::P0))
            .iter()
            .any(|id| {
                e.state()
                    .object(*id)
                    .is_some_and(|object| object.card.is_some_and(|card| card.index == fastbond()))
            })
    );
    assert!(in_graveyard(&e, lk::P1, mirri_s_guile()).is_some());
    assert!(on_battlefield(&e, lk::P1, voidwalker).is_some());
    assert!(on_battlefield(&e, lk::P0, sol_ring()).is_some());
    assert_eq!(
        lk::life(&e, lk::P0),
        24,
        "both successful destructions grant life, including the card exiled instead"
    );
    let gains = e.state().journal.entries()[journal_from..]
        .iter()
        .filter(|entry| {
            matches!(
                entry.event,
                crate::event::GameEvent::LifeChanged { player, old, new, .. }
                    if player == lk::P0 && new > old
            )
        })
        .count();
    assert_eq!(gains, 1, "Decree gains the total life in one event");
}
