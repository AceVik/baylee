//! `cards/enchantments/auras/mv_4/patriarch_s_desire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn patriarch_s_desire() -> CardIndex {
    card_index("23b518ab-561f-4551-8270-082cde99b993")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

/// Enchanted creature gets +2/-2 below Threshold (6 cards in your graveyard)
/// and an additional +2/-2 (total +4/-4) once Threshold reaches 7 cards.
#[test]
fn patriarch_s_desire_gives_base_modifier_and_threshold_bonus_at_seven_cards() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[horned_turtle(), grizzly_bears()],
        &[],
        &[patriarch_s_desire(), dark_ritual()],
        &[],
    );
    let turtle = lk::obj(&e, p0, horned_turtle());
    let bear = lk::obj(&e, p0, grizzly_bears());

    // 6 cards in graveyard (below Threshold).
    seed_graveyard(&mut e, p0, 6);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p0)).len(), 6);

    lk::float(&mut e, p0, &[(ManaColor::Black, 4)]);
    lk::cast_at(&mut e, p0, patriarch_s_desire(), &[turtle], &[]);
    assert_eq!(pt(&e, turtle), (3, 2));
    assert_eq!(pt(&e, bear), (2, 2));

    // 7th card (Dark Ritual) enables Threshold -> additional +2/-2 kills 1/4 Turtle.
    lk::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    lk::cast_at(&mut e, p0, dark_ritual(), &[], &[]);
    assert_eq!(lk::zone_of(&e, turtle), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, bear), Zone::Battlefield);
}
