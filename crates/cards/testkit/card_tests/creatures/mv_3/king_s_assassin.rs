//! `cards/creatures/mv_3/king_s_assassin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn king_s_assassin() -> CardIndex {
    card_index("4e3f89e2-2be6-4b96-b81c-007b6840af82")
}

/// King's Assassin ({1}{B}{B} 1/1 Human Assassin):
/// "{T}: Destroy target tapped creature. Activate only during your turn, before
/// attackers are declared."
#[test]
fn kings_assassin_targets_only_tapped_creatures_and_only_on_your_turn_before_attackers() {
    let mut e = lk::game(
        SEED,
        &[king_s_assassin(), llanowar_elves()],
        &[index::GRIZZLY_BEARS, pearled_unicorn()],
        &[],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    // No tapped creature on the table -> no legal target.
    assert!(!lk::offered(&e, king_s_assassin(), lk::P0, 0));

    // P1 attacks with Grizzly Bears (tapping it); King's Assassin cannot activate on P1's turn.
    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, king_s_assassin(), lk::P0, 0));

    // Back on P0's turn before attackers: tapped Bear is offered, untapped Unicorn is not.
    reach_their_main_phase(&mut e, lk::P0);
    assert!(lk::offered(&e, king_s_assassin(), lk::P0, 0));
    activate(&mut e, lk::P0, king_s_assassin(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target prompt");
    };
    assert!(options.contains(&bear));
    assert!(!options.contains(&unicorn));
    lk::aim(&mut e, &[bear], &[]);
    lk::settle(&mut e);
    assert!(on_battlefield(&e, lk::P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&e, lk::P1, index::GRIZZLY_BEARS).is_some());

    // Even with a tapped creature (elf after attacking), SecondMain is after DeclareAttackers.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, king_s_assassin(), lk::P0, 0));
}
