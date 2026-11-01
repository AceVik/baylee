//! `cards/enchantments/auras/mv_1/incendiary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

fn incendiary() -> CardIndex {
    card_index("a893bee3-ea5c-4619-a875-e4af6d4396fe")
}

/// Incendiary (a893bee3-ea5c-4619-a875-e4af6d4396fe) — "Enchant creature /
/// At the beginning of your upkeep, you may put a fuse counter on this Aura. /
/// When enchanted creature dies, this Aura deals X damage to any target, where
/// X is the number of fuse counters on this Aura."
#[test]
fn incendiary_accumulates_fuse_counters_on_your_upkeep_and_deals_that_much_when_host_dies() {
    let mut e = lk::game(
        SEED,
        &[llanowar_elves()],
        &[],
        &[incendiary(), lightning_bolt()],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, incendiary());
    e.apply(lk::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal host");
    lk::settle(&mut e);
    let aura = lk::obj(&e, lk::P0, incendiary());
    assert_eq!(counters_on(&e, aura, counters::FUSE), 0);

    // P1's upkeep does not trigger Incendiary.
    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(counters_on(&e, aura, counters::FUSE), 0);

    // P0's next upkeep: accept the optional fuse counter -> 1 counter.
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, lk::P0);
    assert_eq!(counters_on(&e, aura, counters::FUSE), 1);

    // Advance one more round and accept a second fuse counter -> 2 counters.
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, lk::P0);
    assert_eq!(counters_on(&e, aura, counters::FUSE), 2);

    // Kill the enchanted Elf; Incendiary triggers and deals 2 damage to P1.
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    lk::aim(&mut e, &[elf], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[], &[lk::P1]);
    lk::settle(&mut e);

    assert_eq!(lk::life(&e, lk::P1), 18, "2 fuse counters dealt 2 damage");
    assert!(in_graveyard(&e, lk::P0, incendiary()).is_some());
}

/// Incendiary — declining the upkeep trigger adds no counter, and bouncing the
/// enchanted creature is not dying so Incendiary deals no damage.
#[test]
fn incendiary_declined_upkeep_and_bounced_host_deal_no_damage() {
    let mut e = lk::game(
        SEED,
        &[llanowar_elves()],
        &[],
        &[incendiary(), unsummon()],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, incendiary());
    e.apply(lk::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    lk::settle(&mut e);
    let aura = lk::obj(&e, lk::P0, incendiary());

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut e, false);
    reach_main_phase(&mut e, lk::P0);
    assert_eq!(counters_on(&e, aura, counters::FUSE), 0);

    e.state
        .object_mut(aura)
        .unwrap()
        .counters
        .set(counters::FUSE, 3);
    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 1)]);
    lk::cast_at(&mut e, lk::P0, unsummon(), &[elf], &[]);

    assert!(in_hand(&e, lk::P0, llanowar_elves()).is_some());
    assert!(in_graveyard(&e, lk::P0, incendiary()).is_some());
    assert_eq!(lk::life(&e, lk::P1), 20);
}
