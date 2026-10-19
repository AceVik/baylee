//! `cards/sorceries/mv_2/soul_exchange.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn soul_exchange() -> CardIndex {
    card_index("366ea3b4-5e7e-4693-9f4b-1e549972609e")
}

fn basal_thrull() -> CardIndex {
    card_index("9da50130-3f83-4968-983c-7dcba257cf1b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Seat 0 holds Soul Exchange and floats {B}{B}; their board and the
/// opponent's are as given. Every card named in `p0_dead` / `p1_dead` is put
/// on the battlefield first and sent to the graveyard before the game starts
/// to matter.
fn board(
    mine: &[CardIndex],
    p0_dead: &[CardIndex],
    p1_dead: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut on_mine = mine.to_vec();
    on_mine.extend_from_slice(p0_dead);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &on_mine)
        .battlefield(1, p1_dead)
        .hand(0, &[soul_exchange()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    for (seat, cards) in [(P0, p0_dead), (P1, p1_dead)] {
        let ids: Vec<ObjectId> = cards
            .iter()
            .map(|c| on_battlefield(&engine, seat, *c).expect("it is on the battlefield"))
            .collect();
        bury(&mut engine, &ids);
    }
    engine.refresh_offer();
    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    engine
}

/// The sacrifice-style cost prompt: the cards that may be exiled.
fn exile_options(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the cost's choice was expected, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    options
}

/// Soul Exchange (366ea3b4-5e7e-4693-9f4b-1e549972609e) — "As an additional
/// cost to cast this spell, exile a creature you control. Return target
/// creature card from your graveyard to the battlefield. Put a +2/+2 counter
/// on that creature if the exiled creature was a Thrull." A Thrull is
/// exiled: the Bears come back as a 4/4.
#[test]
fn soul_exchange_with_a_thrull_returns_the_creature_with_a_counter() {
    let mut engine = board(&[basal_thrull()], &[grizzly_bears()], &[]);
    let thrull = on_battlefield(&engine, P0, basal_thrull()).expect("the Thrull");
    let dead_bears = in_graveyard(&engine, P0, grizzly_bears()).expect("Bears in the graveyard");

    lk::announce(&mut engine, P0, soul_exchange(), 0, 0, &[dead_bears], &[]);
    assert!(
        exile_options(&engine).contains(&thrull),
        "the Thrull may be exiled"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![thrull],
            },
        )
        .expect("exiling the Thrull pays the cost");
    assert_eq!(
        lk::zone_of(&engine, thrull),
        Zone::Exile,
        "exiled as a cost"
    );
    lk::settle(&mut engine);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).expect("the Bears are back");
    assert_eq!(pt(&engine, bears), (4, 4), "2/2 and a +2/+2 counter");
    assert!(in_graveyard(&engine, P0, soul_exchange()).is_some());
}

/// Soul Exchange — "if the exiled creature was a Thrull": exiling anything
/// else brings the creature back plain.
#[test]
fn soul_exchange_without_a_thrull_returns_the_creature_plain() {
    let mut engine = board(&[quiet_creature()], &[grizzly_bears()], &[]);
    let fodder = on_battlefield(&engine, P0, quiet_creature()).expect("the fodder");
    let dead_bears = in_graveyard(&engine, P0, grizzly_bears()).expect("Bears in the graveyard");

    lk::announce(&mut engine, P0, soul_exchange(), 0, 0, &[dead_bears], &[]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("exiling a creature pays the cost");
    assert_eq!(lk::zone_of(&engine, fodder), Zone::Exile);
    lk::settle(&mut engine);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).expect("the Bears are back");
    assert_eq!(pt(&engine, bears), (2, 2), "no counter");
}

/// Soul Exchange — "target creature card from your graveyard": the
/// opponent's graveyard is not offered.
#[test]
fn soul_exchange_only_targets_creature_cards_in_your_own_graveyard() {
    let mut engine = board(&[basal_thrull()], &[grizzly_bears()], &[hill_giant()]);
    let mine = in_graveyard(&engine, P0, grizzly_bears()).expect("my Bears");
    let theirs = in_graveyard(&engine, P1, hill_giant()).expect("their Giant");

    cast_with_floating(&mut engine, P0, soul_exchange());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert!(options.contains(&mine), "my graveyard's creature");
    assert!(!options.contains(&theirs), "not the opponent's");
}

/// Soul Exchange — the exile is a cost: with no creature to exile it cannot
/// be cast, though there is a creature card to return.
#[test]
fn soul_exchange_needs_a_creature_to_exile() {
    let engine = board(&[], &[grizzly_bears()], &[]);
    assert!(
        !lk::castable(&engine, P0, soul_exchange()),
        "no creature on the battlefield to exile"
    );
}
