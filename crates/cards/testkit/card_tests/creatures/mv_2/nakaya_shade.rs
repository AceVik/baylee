//! `cards/creatures/mv_2/nakaya_shade.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn nakaya_shade() -> CardIndex {
    card_index("ce969713-fdaa-4c31-b164-76a9b0b9ff81")
}

/// Nakaya Shade ({1}{B} 1/1 Shade):
/// "{B}: This creature gets +1/+1 until end of turn unless any player pays {2}."
#[test]
fn nakaya_shade_grows_when_nobody_pays_two_and_is_prevented_when_opponent_pays() {
    let mut e = lk::game(SEED, &[nakaya_shade()], &[plains(), plains()], &[], &[]);
    let shade = lk::obj(&e, lk::P0, nakaya_shade());

    // First activation: P0 has {B}{B}{B} (can pay {2} after spending {B}), P1 has 2 untapped Plains.
    // Both decline to pay {2} -> Shade becomes 2/2.
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 3)]);
    activate(&mut e, lk::P0, nakaya_shade(), 0);
    pass_until(&mut e, |eng| matches!(eng.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    pass_until(&mut e, |eng| matches!(eng.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(pt(&e, shade), (2, 2));

    // Second activation: P0 has only {B} left, P1 pays {2} -> Shade stays 2/2 instead of 3/3!
    lk::float(&mut e, lk::P1, &[(ManaColor::White, 2)]);
    activate(&mut e, lk::P0, nakaya_shade(), 0);
    pass_until(&mut e, |eng| matches!(eng.pending(), Pending::YesNo { .. }));
    while let Pending::YesNo { player, .. } = e.pending().clone() {
        e.apply(player, PlayerAction::YesNo(player == lk::P1))
            .unwrap();
        if stack_is_empty(&e) {
            break;
        }
    }
    lk::settle(&mut e);
    assert_eq!(pt(&e, shade), (2, 2));

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, shade), (1, 1));
}
