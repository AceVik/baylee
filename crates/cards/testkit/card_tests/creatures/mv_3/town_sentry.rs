//! `cards/creatures/mv_3/town_sentry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn town_sentry() -> CardIndex {
    card_index("ec1ac4df-b031-43b4-a2de-ca58a1599053")
}

/// Town Sentry ({2}{W} 2/2 Human Soldier):
/// "Whenever this creature blocks, it gets +0/+2 until end of turn."
#[test]
fn town_sentry_gets_plus_zero_plus_two_only_when_blocking_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[town_sentry()],
        &[llanowar_elves()],
        &[index::SHOCK],
        &[],
    );
    let sentry = lk::obj(&e, lk::P0, town_sentry());
    let elf = lk::obj(&e, lk::P1, llanowar_elves());

    // Attacking does not trigger Town Sentry.
    let _ = lk::attack(&mut e, &[sentry]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(pt(&e, sentry), (2, 2));

    // Untap on P0's next turn, then pass to P1's attack and block Elf (1/1):
    // +0/+2 makes Sentry 2/4 so 1 combat damage + 2 Shock damage (= 3) is nonlethal!
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[(sentry, elf)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, sentry), (2, 4));

    lk::through_combat(&mut e);
    assert_eq!(pt(&e, sentry), (2, 4));
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::SHOCK, &[sentry], &[]);
    assert!(on_battlefield(&e, lk::P0, town_sentry()).is_some());

    reach_their_main_phase(&mut e, lk::P0);
    assert_eq!(pt(&e, sentry), (2, 2));
}
