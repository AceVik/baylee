//! `cards/enchantments/mv_4/awakening.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn awakening() -> CardIndex {
    card_index("5563427d-ffdc-4bea-ad7d-1605894b8a6c")
}

/// Awakening (`5563427d-ffdc-4bea-ad7d-1605894b8a6c`) — {2}{G}{G} Enchantment (Implemented):
/// "At the beginning of each upkeep, untap all creatures and lands."
#[test]
fn awakening_untaps_creatures_and_lands_at_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(0, &[awakening()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, awakening());
    pass_until(&mut engine, stack_is_empty);

    let elf = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    tap_all_mana(&mut engine, P0);
    assert!(is_tapped(&engine, elf));

    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1)
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
    });

    assert!(!is_tapped(&engine, elf));
}

#[test]
fn upkeep_untaps_creatures_and_lands_on_both_sides_but_not_artifacts() {
    let mut e = lk::game(
        SEED,
        &[awakening(), index::GRIZZLY_BEARS, forest(), index::SOL_RING],
        &[index::GRIZZLY_BEARS, island()],
        &[],
        &[],
    );
    let mine = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let my_land = lk::obj(&e, P0, forest());
    let artifact = lk::obj(&e, P0, index::SOL_RING);
    let theirs = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let their_land = lk::obj(&e, lk::P1, island());
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && !stack_is_empty(e)
    });
    for id in [mine, my_land, artifact, theirs, their_land] {
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }
    lk::settle(&mut e);
    for id in [mine, my_land, theirs, their_land] {
        assert!(!lk::tapped(&e, id));
    }
    assert!(lk::tapped(&e, artifact));
}
