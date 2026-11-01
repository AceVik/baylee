//! `cards/creatures/mv_3/gempalm_incinerator.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn gempalm() -> CardIndex {
    card_index("467ea3e3-7767-405a-9d1a-8fa5de696376")
}

#[test]
fn gempalm_incinerator_cycles_draws_and_counts_goblins_on_both_sides() {
    let mut e = lk::game(
        SEED,
        &[index::RAGING_GOBLIN, mountain(), mountain()],
        &[index::GOBLIN_SKY_RAIDER],
        &[gempalm()],
        &[],
    );
    let victim = on_battlefield(&e, lk::P1, index::GOBLIN_SKY_RAIDER).unwrap();
    let hand = e.state().zones.list(ZoneLocation::Hand(lk::P0)).len();
    tap_all_mana(&mut e, lk::P0);
    activate(&mut e, lk::P0, gempalm(), 0);
    assert!(in_graveyard(&e, lk::P0, gempalm()).is_some());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[victim], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P1, index::GOBLIN_SKY_RAIDER).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), hand);
}

#[test]
fn gempalm_incinerator_damage_is_optional() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain()],
        &[index::GOBLIN_SKY_RAIDER],
        &[gempalm()],
        &[],
    );
    let victim = on_battlefield(&e, lk::P1, index::GOBLIN_SKY_RAIDER).unwrap();
    tap_all_mana(&mut e, lk::P0);
    activate(&mut e, lk::P0, gempalm(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[victim], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, victim), 0);
}
