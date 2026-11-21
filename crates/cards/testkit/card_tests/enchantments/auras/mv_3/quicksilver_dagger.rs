//! `cards/enchantments/auras/mv_3/quicksilver_dagger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn quicksilver_dagger() -> CardIndex {
    card_index("5c77bf76-bb0f-4e44-babe-0e7b7b555e1f")
}

fn jace_the_mind_sculptor() -> CardIndex {
    card_index("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
}

/// Enchanted creature has "{T}: This creature deals 1 damage to target player
/// or planeswalker. You draw a card." (cannot target a creature).
#[test]
fn quicksilver_dagger_pings_player_or_planeswalker_and_draws_for_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[jace_the_mind_sculptor(), llanowar_elves()],
        &[quicksilver_dagger()],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let jace = lk::obj(&e, p1, jace_the_mind_sculptor());

    lk::float(
        &mut e,
        p0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
        ],
    );
    lk::cast_at(&mut e, p0, quicksilver_dagger(), &[bear], &[]);

    // Ping P1 -> P1 drops 20 -> 19, P0 draws a card.
    let lib0 = library_size(&e, p0);
    lk::use_ability(
        &mut e,
        p0,
        grizzly_bears(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[],
        &[p1],
    );
    assert_eq!(lk::life(&e, p1), 19);
    assert_eq!(library_size(&e, p0), lib0 - 1);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);

    // Next turn, ping Jace -> Jace loses 1 loyalty (3 -> 2), P0 draws again.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::use_ability(
        &mut e,
        p0,
        grizzly_bears(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[jace],
        &[],
    );
    assert_eq!(
        e.state()
            .object(jace)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        2
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 3);
}
