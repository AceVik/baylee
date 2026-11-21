//! `cards/enchantments/auras/mv_2/squee_s_embrace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn squee_s_embrace() -> CardIndex {
    card_index("095f8dac-15b8-4c28-ae33-6dc71152bcc7")
}

/// Enchanted creature gets +2/+2 and returns to its owner's hand when it dies,
/// but not when exiled; enchanting an opponent's creature returns it to them.
#[test]
fn squee_s_embrace_buffs_and_returns_on_death_only_to_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[llanowar_elves()],
        &[
            squee_s_embrace(),
            squee_s_embrace(),
            lightning_bolt(),
            lightning_bolt(),
            swords_to_plowshares(),
        ],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let elves = lk::obj(&e, p1, llanowar_elves());

    lk::float(&mut e, p0, &[(ManaColor::Red, 2), (ManaColor::White, 2)]);
    lk::cast_at(&mut e, p0, squee_s_embrace(), &[bear], &[]);
    lk::cast_at(&mut e, p0, squee_s_embrace(), &[elves], &[]);
    assert_eq!(pt(&e, bear), (4, 4));
    assert_eq!(pt(&e, elves), (3, 3));

    // Killing P1's Elves with Bolt returns them to P1's (owner's) hand.
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[elves], &[]);
    assert!(in_hand(&e, p1, llanowar_elves()).is_some());
    assert!(in_hand(&e, p0, llanowar_elves()).is_none());

    // Exiling Bear with Swords does NOT trigger return to hand.
    lk::float(&mut e, p0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut e, p0, swords_to_plowshares(), &[bear], &[]);
    assert_eq!(lk::zone_of(&e, bear), Zone::Exile);
    assert!(in_hand(&e, p0, grizzly_bears()).is_none());
}
