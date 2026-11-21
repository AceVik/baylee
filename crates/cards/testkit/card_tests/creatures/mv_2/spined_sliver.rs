//! `cards/creatures/mv_2/spined_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn spined_sliver() -> CardIndex {
    card_index("eed9c168-1c78-43c6-a814-9b80054d1373")
}

/// Spined Sliver ({R}{G} 2/2 Sliver):
/// "Whenever a Sliver becomes blocked, that Sliver gets +1/+1 until end of turn
/// for each creature blocking it."
#[test]
fn spined_sliver_pumps_blocked_sliver_by_blocker_count_and_skips_unblocked_or_nonsliver() {
    let mut e = lk::game(
        SEED,
        &[
            spined_sliver(),
            index::METALLIC_SLIVER,
            index::GRIZZLY_BEARS,
        ],
        &[llanowar_elves(), llanowar_elves(), pearled_unicorn()],
        &[],
        &[],
    );
    let spined = lk::obj(&e, lk::P0, spined_sliver());
    let metallic = lk::obj(&e, lk::P0, index::METALLIC_SLIVER);
    let bear = lk::obj(&e, lk::P0, index::GRIZZLY_BEARS);
    let p1_elves: Vec<ObjectId> = e
        .state()
        .battlefield_seen()
        .filter(|&id| {
            let o = e.state().object(id).unwrap();
            o.controller == lk::P1 && o.card.is_some_and(|c| c.index == llanowar_elves())
        })
        .collect();
    let unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    let _ = lk::attack(&mut e, &[spined, metallic, bear]);
    lk::block(
        &mut e,
        &[
            (p1_elves[0], spined),
            (p1_elves[1], spined),
            (unicorn, bear),
        ],
    );
    // Resolve Spined Sliver's single trigger (+2/+2 for 2 blockers -> 4/4).
    lk::settle(&mut e);
    assert_eq!(pt(&e, spined), (4, 4));
    assert_eq!(pt(&e, metallic), (1, 1));
    assert_eq!(pt(&e, bear), (2, 2));

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseNumber { .. })
            || eng.state().turn.phase == Phase::SecondMain
    });
    if let Pending::ChooseNumber { player, .. } = e.pending().clone() {
        e.apply(player, PlayerAction::ChooseNumber(2)).unwrap();
    }
    lk::through_combat(&mut e);
    assert_eq!(pt(&e, spined), (4, 4));
    assert_eq!(lk::life(&e, lk::P1), 19);

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, spined), (2, 2));
}

#[test]
fn spined_sliver_is_symmetric_and_pumps_an_opponents_blocked_sliver() {
    let mut e = lk::game(
        SEED,
        &[spined_sliver(), llanowar_elves(), pearled_unicorn()],
        &[index::METALLIC_SLIVER],
        &[],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let their_sliver = lk::obj(&e, lk::P1, index::METALLIC_SLIVER);

    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[their_sliver]);
    lk::block(&mut e, &[(elf, their_sliver), (unicorn, their_sliver)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, their_sliver), (3, 3));
}
