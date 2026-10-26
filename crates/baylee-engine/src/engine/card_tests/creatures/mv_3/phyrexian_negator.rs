//! `cards/creatures/mv_3/phyrexian_negator.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn phyrexian_negator() -> CardIndex {
    card_index("f22fcd17-bc24-41df-81ce-6d11208861b9")
}

/// Phyrexian Negator (f22fcd17-bc24-41df-81ce-6d11208861b9) — 5/5 with
/// "Trample / Whenever this creature is dealt damage, sacrifice that many
/// permanents."
#[test]
fn phyrexian_negator_tramples_and_sacrifices_that_many_permanents_when_dealt_damage() {
    let mut e = lk::game(
        SEED,
        &[phyrexian_negator(), sol_ring(), llanowar_elves(), forest()],
        &[sol_ring()],
        &[lightning_bolt()],
        &[],
    );
    let negator = lk::obj(&e, lk::P0, phyrexian_negator());
    let their_ring = lk::obj(&e, lk::P1, sol_ring());

    assert_eq!(pt(&e, negator), (5, 5));
    assert!(keywords(&e, negator).contains(KeywordSet::TRAMPLE));

    // Deal 3 damage to Phyrexian Negator with Lightning Bolt.
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    lk::aim(&mut e, &[negator], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // Keep Negator and sacrifice the other three permanents together.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        panic!("expected permanent choice, got {:?}", e.pending());
    };
    assert_eq!(player, lk::P0);
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&negator));
    assert!(
        !options.contains(&their_ring),
        "opponent's permanent cannot be sacrificed"
    );
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![negator],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P0, sol_ring()).is_some());
    assert!(in_graveyard(&e, lk::P0, llanowar_elves()).is_some());
    assert!(in_graveyard(&e, lk::P0, forest()).is_some());
    assert!(
        on_battlefield(&e, lk::P0, phyrexian_negator()).is_some(),
        "Negator survived 3 damage after sacrificing the other 3 permanents"
    );
}

/// Phyrexian Negator — damage dealt to another object or player does not
/// trigger Phyrexian Negator's sacrifice ability.
#[test]
fn phyrexian_negator_does_not_trigger_when_damage_is_dealt_elsewhere() {
    let mut e = lk::game(
        SEED,
        &[phyrexian_negator(), sol_ring()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[], &[lk::P1]);

    assert_eq!(lk::life(&e, lk::P1), 17);
    assert!(on_battlefield(&e, lk::P0, phyrexian_negator()).is_some());
    assert!(on_battlefield(&e, lk::P0, sol_ring()).is_some());
}

#[test]
fn phyrexian_negator_sacrifices_simultaneously_with_a_dies_trigger_source() {
    let fecundity = card_index("ffa64ac6-fe55-48b8-b015-849982cd7ad8");
    let mut e = lk::game(
        SEED,
        &[phyrexian_negator(), fecundity, llanowar_elves(), sol_ring()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let negator = lk::obj(&e, lk::P0, phyrexian_negator());
    let hand_before = e.state().zones.list(ZoneLocation::Hand(lk::P0)).len();
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    lk::aim(&mut e, &[negator], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![negator],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(in_graveyard(&e, lk::P0, fecundity).is_some());
    assert!(in_graveyard(&e, lk::P0, llanowar_elves()).is_some());
    e.apply(lk::P0, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(),
        hand_before,
        "Fecundity observes the simultaneously sacrificed Elf despite also leaving"
    );
}
