//! `cards/enchantments/mv_4/sigil_of_the_new_dawn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sigil_of_the_new_dawn() -> CardIndex {
    card_index("cb029c0f-b08a-4952-b236-8166b9f17119")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

/// Whenever a creature goes to your graveyard from the battlefield (even if an
/// opponent controlled it), you may pay {1}{W} to return it to your hand.
#[test]
fn sigil_of_the_new_dawn_returns_owned_dying_creatures_on_optional_payment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[sigil_of_the_new_dawn(), llanowar_elves(), grizzly_bears()],
        &[llanowar_elves()],
        &[lightning_bolt(), lightning_bolt(), lightning_bolt()],
        &[control_magic()],
    );
    let my_elf = lk::obj(&e, p0, llanowar_elves());
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let opp_elf = lk::obj(&e, p1, llanowar_elves());

    // 1. Opponent's creature dies -> Sigil does NOT trigger.
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[opp_elf], &[]);
    assert_eq!(lk::zone_of(&e, opp_elf), Zone::Graveyard);

    // 2. P0's Elf dies, P0 declines {1}{W} -> stays in graveyard.
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, p0, lightning_bolt());
    lk::aim(&mut e, &[my_elf], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, my_elf), Zone::Graveyard);

    // 3. P1 steals P0's Bear with Control Magic, then P0 kills Bear -> goes to P0's yard & returns to P0's hand!
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Blue, 4)]);
    lk::cast_at(&mut e, p1, control_magic(), &[my_bear], &[]);
    reach_their_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &[(ManaColor::Red, 1), (ManaColor::White, 2)]);
    cast_with_floating(&mut e, p0, lightning_bolt());
    lk::aim(&mut e, &[my_bear], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert!(in_hand(&e, p0, grizzly_bears()).is_some());
}
