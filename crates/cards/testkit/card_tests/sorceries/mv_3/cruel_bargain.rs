//! `cards/sorceries/mv_3/cruel_bargain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cruel_bargain() -> CardIndex {
    card_index("dcf105c1-d37b-4e92-a420-9872a2187764")
}

/// Cruel Bargain (dcf105c1-d37b-4e92-a420-9872a2187764) — {B}{B}{B} Sorcery:
/// "Draw four cards. You lose half your life, rounded up." At 19 life, half
/// rounded up is 10 life lost (leaving 9), and four cards are drawn.
#[test]
fn cruel_bargain_draws_four_cards_and_loses_half_life_rounded_up() {
    let mut odd = Duel::new(SEED, forest())
        .life(0, 19)
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[cruel_bargain()])
        .start();
    keep_mulligans(&mut odd);
    reach_main_phase(&mut odd, P0);

    let lib_before = library_size(&odd, P0);
    let hand_before = odd.state().zones.list(ZoneLocation::Hand(P0)).len();

    cast_from_hand(&mut odd, P0, cruel_bargain());
    lk::settle(&mut odd);

    assert_eq!(library_size(&odd, P0), lib_before - 4, "drew four cards");
    assert_eq!(
        odd.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 1 + 4
    );
    assert_eq!(
        lk::life(&odd, P0),
        9,
        "ceil(19 / 2) = 10 life lost, leaving 9"
    );
    assert_eq!(lk::life(&odd, P1), 20);
    assert!(in_graveyard(&odd, P0, cruel_bargain()).is_some());

    let mut even = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[],
        &[cruel_bargain()],
        &[],
    );
    cast_from_hand(&mut even, P0, cruel_bargain());
    lk::settle(&mut even);
    assert_eq!(lk::life(&even, P0), 10, "20 / 2 = 10 life lost");
}
