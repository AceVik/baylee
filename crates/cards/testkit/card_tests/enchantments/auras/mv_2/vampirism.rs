//! `cards/enchantments/auras/mv_2/vampirism.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn vampirism() -> CardIndex {
    card_index("14df239a-c7cc-492a-94f5-2971f6efe17b")
}

/// Enchanted creature gets +1/+1 for each other creature you control, other
/// creatures you control get -1/-1 (opponents' untouched), and ETB draws a card
/// at the beginning of the next turn's upkeep (once only).
#[test]
fn vampirism_pumps_host_shrinks_your_others_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[
            grizzly_bears(),
            gray_ogre(),
            shock_troops(),
            llanowar_elves(),
        ],
        &[grizzly_bears()],
        &[vampirism()],
        &[],
    );
    let host = lk::obj(&e, p0, grizzly_bears());
    let ogre = lk::obj(&e, p0, gray_ogre());
    let troops = lk::obj(&e, p0, shock_troops());
    let elves = lk::obj(&e, p0, llanowar_elves());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::Black, 2)]);
    lk::cast_at(&mut e, p0, vampirism(), &[host], &[]);

    // 1/1 Llanowar Elves dies to -1/-1; Ogre & Shock Troops become 1/1 & 1/1;
    // host gets +2/+2 from those two surviving other creatures; opponent's is 2/2.
    assert_eq!(lk::zone_of(&e, elves), Zone::Graveyard);
    assert_eq!(pt(&e, ogre), (1, 1));
    assert_eq!(pt(&e, troops), (1, 1));
    assert_eq!(pt(&e, host), (4, 4));
    assert_eq!(pt(&e, opp_bear), (2, 2));

    // Next turn's upkeep (opponent's upkeep) triggers the delayed draw for P0.
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut e, p1);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );

    // P0's subsequent upkeep draws only P0's normal draw step card, no extra.
    reach_their_main_phase(&mut e, p0);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}
