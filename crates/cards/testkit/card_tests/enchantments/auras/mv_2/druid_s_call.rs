//! `cards/enchantments/auras/mv_2/druid_s_call.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn druid_s_call() -> CardIndex {
    card_index("e71f4fe8-2ecd-450b-8c41-5e38d749f59d")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

fn creature_count(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            e.state().object(id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .count()
}

/// Whenever the enchanted creature is dealt damage, ITS controller creates that
/// many 1/1 green Squirrel tokens (verified for both your creature and opponent's).
#[test]
fn druid_s_call_creates_squirrels_for_enchanted_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[horned_turtle()],
        &[horned_turtle(), llanowar_elves()],
        &[
            druid_s_call(),
            druid_s_call(),
            lightning_bolt(),
            lightning_bolt(),
        ],
        &[],
    );
    let my_turtle = lk::obj(&e, p0, horned_turtle());
    let opp_turtle = lk::obj(&e, p1, horned_turtle());

    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, p0, druid_s_call(), &[my_turtle], &[]);
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, p0, druid_s_call(), &[opp_turtle], &[]);

    // 3 damage to your Turtle -> 3 Squirrels for P0 (1 -> 4 creatures).
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[my_turtle], &[]);
    assert_eq!(creature_count(&e, p0), 4);
    assert_eq!(creature_count(&e, p1), 2);

    // 3 damage to P1's Turtle -> 3 Squirrels for P1 (2 -> 5 creatures), NOT P0.
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[opp_turtle], &[]);
    assert_eq!(creature_count(&e, p0), 4);
    assert_eq!(creature_count(&e, p1), 5);
}
