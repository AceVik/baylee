//! `cards/enchantments/mv_3/overwhelming_instinct.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn overwhelming_instinct() -> CardIndex {
    card_index("9ba89c25-5302-414f-a2ec-436b8981999d")
}

/// Attacking with 2 creatures draws 0 cards; attacking with 3+ draws 1 card;
/// opponent attacking with 3+ draws 0 cards for you.
#[test]
fn overwhelming_instinct_draws_once_when_you_attack_with_three_or_more() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[
            overwhelming_instinct(),
            llanowar_elves(),
            llanowar_elves(),
            llanowar_elves(),
            llanowar_elves(),
        ],
        &[llanowar_elves(), llanowar_elves(), llanowar_elves()],
        &[],
        &[],
    );
    let my_elves = all_on_battlefield(&e, p0, llanowar_elves());

    // Attack with 2 -> hand stays empty.
    lk::attack(&mut e, &my_elves[..2]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 0);

    // Opponent attacks with 3 -> P0 draws 0.
    reach_their_main_phase(&mut e, p1);
    let opp_elves = all_on_battlefield(&e, p1, llanowar_elves());
    lk::attack(&mut e, &opp_elves);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 0);

    // P0 attacks with 4 -> draws 1 card for Overwhelming Instinct (plus 1 turn draw = 2).
    reach_their_main_phase(&mut e, p0);
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    lk::attack(&mut e, &my_elves);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
}
