//! `cards/creatures/mv_2/skirk_drill_sergeant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn sergeant() -> CardIndex {
    card_index("eb0d97a3-6494-4011-9f60-1b12b165a995")
}

fn die(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    bury(e, &[id]);
}

fn pay_reveal(e: &mut Engine<RegistryLookup>, pay: bool) {
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(e, pay);
    lk::settle(e);
}

#[test]
fn skirk_drill_sergeant_recruits_only_goblin_permanents_when_paid() {
    let mut e = lk::game(
        SEED,
        &[sergeant(), index::RAGING_GOBLIN, index::RAGING_GOBLIN],
        &[],
        &[
            index::GOBLIN_SKY_RAIDER,
            index::SHOCK,
            index::BOGGART_SHENANIGANS,
        ],
        &[],
    );
    // Reverse these moves so the revealed order is Raider, Shock, Shenanigans.
    hand_to_library_top(&mut e, lk::P0, index::BOGGART_SHENANIGANS);
    hand_to_library_top(&mut e, lk::P0, index::SHOCK);
    hand_to_library_top(&mut e, lk::P0, index::GOBLIN_SKY_RAIDER);

    let goblins = all_on_battlefield(&e, lk::P0, index::RAGING_GOBLIN);
    die(&mut e, goblins[0]);
    let before = library_size(&e, lk::P0);
    pay_reveal(&mut e, false);
    assert_eq!(
        library_size(&e, lk::P0),
        before,
        "declining reveals nothing"
    );

    die(&mut e, goblins[1]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    pay_reveal(&mut e, true);
    assert!(on_battlefield(&e, lk::P0, index::GOBLIN_SKY_RAIDER).is_some());
    assert_eq!(lk::pool_total(&e, lk::P0), 0);

    let source = lk::obj(&e, lk::P0, sergeant());
    die(&mut e, source);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    pay_reveal(&mut e, true);
    assert!(in_graveyard(&e, lk::P0, index::SHOCK).is_some());
}

#[test]
fn skirk_drill_sergeant_puts_a_goblin_kindred_card_onto_the_battlefield() {
    let mut e = lk::game(
        SEED,
        &[sergeant(), index::RAGING_GOBLIN],
        &[],
        &[index::BOGGART_SHENANIGANS],
        &[],
    );
    hand_to_library_top(&mut e, lk::P0, index::BOGGART_SHENANIGANS);
    let goblin = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    die(&mut e, goblin);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    pay_reveal(&mut e, true);
    assert!(on_battlefield(&e, lk::P0, index::BOGGART_SHENANIGANS).is_some());
}

#[test]
fn skirk_drill_sergeant_does_not_trigger_for_a_non_goblin() {
    let mut e = lk::game(SEED, &[sergeant(), grizzly_bears()], &[], &[], &[]);
    let bears = lk::obj(&e, lk::P0, grizzly_bears());
    die(&mut e, bears);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert_eq!(library_size(&e, lk::P0), 60);
}
