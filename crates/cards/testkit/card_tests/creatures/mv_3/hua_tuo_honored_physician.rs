//! `cards/creatures/mv_3/hua_tuo_honored_physician.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;
use baylee_engine::zone::ZonePosition;

fn hua_tuo() -> CardIndex {
    card_index("00284955-0a1b-4939-9567-a7d34cbcee51")
}

/// Hua Tuo, Honored Physician ({1}{G}{G} 1/2 Legendary Human):
/// "{T}: Put target creature card from your graveyard on top of your library.
/// Activate only during your turn, before attackers are declared."
#[test]
fn hua_tuo_puts_creature_from_your_graveyard_on_top_of_library_before_attackers() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[hua_tuo(), llanowar_elves()])
        .hand(0, &[index::GRIZZLY_BEARS, index::SHOCK])
        .hand(1, &[pearled_unicorn()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, lk::P0);

    let my_bear = in_hand(&e, lk::P0, index::GRIZZLY_BEARS).unwrap();
    let my_shock = in_hand(&e, lk::P0, index::SHOCK).unwrap();
    let their_unicorn = in_hand(&e, lk::P1, pearled_unicorn()).unwrap();
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    {
        let state = e.dev_state_mut(lk::P0).unwrap();
        for (id, seat) in [
            (my_bear, lk::P0),
            (my_shock, lk::P0),
            (their_unicorn, lk::P1),
        ] {
            state
                .move_object(
                    id,
                    ZoneLocation::Graveyard(seat),
                    ZonePosition::Top,
                    baylee_engine::event::Cause::Setup,
                )
                .unwrap();
        }
    }
    e.refresh_offer();

    // Post-attackers and opponent's turn are illegal.
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, hua_tuo(), lk::P0, 0));
    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, hua_tuo(), lk::P0, 0));

    // In P0's next Upkeep, put Grizzly Bears on top and draw it in the draw step.
    lk::to_step(&mut e, lk::P0, Step::Upkeep);
    activate(&mut e, lk::P0, hua_tuo(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![my_bear]);
    lk::aim(&mut e, &[my_bear], &[]);
    lk::settle(&mut e);

    reach_main_phase(&mut e, lk::P0);
    assert!(in_hand(&e, lk::P0, index::GRIZZLY_BEARS).is_some());
}
