//! `cards/enchantments/kindred/mv_3/boggart_shenanigans.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;

fn boggart_shenanigans() -> CardIndex {
    card_index("997d2f38-5a50-451e-bd48-e36dcb24d967")
}

#[test]
fn boggart_shenanigans_watches_another_goblin_you_control_and_may_burn_a_walker() {
    let mut e = lk::game(
        SEED,
        &[boggart_shenanigans(), index::RAGING_GOBLIN, grizzly_bears()],
        &[index::RAGING_GOBLIN, index::JACE_THE_MIND_SCULPTOR],
        &[],
        &[],
    );
    let walker = lk::obj(&e, lk::P1, index::JACE_THE_MIND_SCULPTOR);
    e.dev_state_mut(lk::P0)
        .unwrap()
        .object_mut(walker)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 4);
    e.refresh_offer();
    let non_goblin = lk::obj(&e, lk::P0, grizzly_bears());
    let opposing_goblin = lk::obj(&e, lk::P1, index::RAGING_GOBLIN);
    kill(&mut e, non_goblin);
    kill(&mut e, opposing_goblin);
    assert!(
        stack_is_empty(&e),
        "a non-Goblin and an opponent's Goblin do not trigger"
    );

    let our_goblin = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    kill(&mut e, our_goblin);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!(
            "the trigger targets a player or planeswalker: {:?}",
            e.pending()
        );
    };
    assert!(options.contains(&walker));
    assert!(player_options.contains(&lk::P1));
    lk::aim(&mut e, &[walker], &[]);
    lk::pass_once(&mut e);
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert_eq!(
        e.state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        3
    );
}

#[test]
fn boggart_shenanigans_may_decline_the_damage() {
    let mut e = lk::game(
        SEED,
        &[boggart_shenanigans(), index::RAGING_GOBLIN],
        &[],
        &[],
        &[],
    );
    let goblin = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    kill(&mut e, goblin);
    lk::aim(&mut e, &[], &[lk::P1]);
    lk::pass_once(&mut e);
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 20);
}
