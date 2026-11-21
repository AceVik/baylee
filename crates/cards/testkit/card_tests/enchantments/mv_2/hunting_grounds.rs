//! `cards/enchantments/mv_2/hunting_grounds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn hunting_grounds() -> CardIndex {
    card_index("2f3fec87-3dd2-4e5d-835c-eae535da4ecb")
}

fn garruk_s_companion() -> CardIndex {
    card_index("32fc8fa7-7e0a-4d4e-85cf-2f98b3fc6ecf")
}

/// Inactive with 6 graveyard cards; once Threshold (7+) is met, opponent casts
/// optionally let you put a creature card from your hand onto the battlefield.
#[test]
fn hunting_grounds_requires_threshold_and_puts_creature_on_opponent_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[hunting_grounds()],
        &[],
        &[dark_ritual(), garruk_s_companion()],
        &[llanowar_elves(), llanowar_elves(), grizzly_bears()],
    );

    // 6 cards in P0's graveyard -> opponent casting does not trigger Hunting Grounds.
    seed_graveyard(&mut e, p0, 6);
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut e, p1, llanowar_elves(), &[], &[]);
    assert!(on_battlefield(&e, p0, garruk_s_companion()).is_none());

    // Reach 7 cards via Dark Ritual; casting your own spell doesn't trigger it.
    reach_their_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    lk::cast_at(&mut e, p0, dark_ritual(), &[], &[]);
    assert!(on_battlefield(&e, p0, garruk_s_companion()).is_none());

    // Opponent casts Elf -> decline MayDo; opponent casts Bear -> accept & put Companion.
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut e, p1, llanowar_elves());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert!(in_hand(&e, p0, garruk_s_companion()).is_some());

    cast_with_floating(&mut e, p1, grizzly_bears());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let companion = in_hand(&e, p0, garruk_s_companion()).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![companion],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, companion), Zone::Battlefield);
}
