//! `cards/enchantments/mv_5/aether_charge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn aether_charge() -> CardIndex {
    card_index("15e83068-6253-4c65-8679-7295f3dc2075")
}

fn garruk_s_companion() -> CardIndex {
    card_index("32fc8fa7-7e0a-4d4e-85cf-2f98b3fc6ecf")
}

fn jace_the_mind_sculptor() -> CardIndex {
    card_index("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
}

/// When a Beast you control enters, optionally deal 4 damage to target opponent
/// or planeswalker; non-Beasts and opponent Beasts do not trigger it.
#[test]
fn aether_charge_triggers_for_your_beasts_and_deals_optional_four_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[aether_charge()],
        &[jace_the_mind_sculptor()],
        &[
            grizzly_bears(),
            garruk_s_companion(),
            garruk_s_companion(),
            garruk_s_companion(),
        ],
        &[garruk_s_companion()],
    );
    let jace = lk::obj(&e, p1, jace_the_mind_sculptor());

    // Non-Beast entering under P0 doesn't trigger.
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, p0, grizzly_bears(), &[], &[]);
    assert_eq!(lk::life(&e, p1), 20);

    // 1st Beast: decline "you may" -> deals 0 damage.
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, p0, garruk_s_companion());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[], &[p1]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p1), 20);

    // 2nd Beast: accept -> deals 4 damage to P1 (20 -> 16).
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, p0, garruk_s_companion());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[], &[p1]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p1), 16);

    // 3rd Beast: accept -> deals 4 damage to Jace (loyalty 3 -> graveyard).
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, p0, garruk_s_companion());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[jace], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, jace), Zone::Graveyard);

    // Opponent's Beast does NOT trigger Aether Charge.
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, p1, garruk_s_companion(), &[], &[]);
    assert_eq!(lk::life(&e, p0), 20);
}
