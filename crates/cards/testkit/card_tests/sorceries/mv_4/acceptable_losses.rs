//! `cards/sorceries/mv_4/acceptable_losses.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn acceptable_losses() -> CardIndex {
    card_index("2b6d95b5-0680-43ae-9ff3-8e287a9eb837")
}

/// Acceptable Losses (2b6d95b5-0680-43ae-9ff3-8e287a9eb837) — {3}{R} Sorcery:
/// "As an additional cost to cast this spell, discard a card at random.
/// Acceptable Losses deals 5 damage to target creature."
#[test]
fn acceptable_losses_discards_random_card_on_cast_and_deals_five_to_creature() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[fangren_hunter()],
        &[acceptable_losses(), forest(), island()],
        &[],
    );
    let hunter = lk::obj(&engine, P1, fangren_hunter());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, acceptable_losses());
    aim_at(&mut engine, P0, hunter);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        1,
        "one of the two remaining cards discarded at random as additional cost"
    );
    assert_eq!(lk::zone_of(&engine, hunter), Zone::Battlefield);

    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, hunter), Zone::Graveyard);

    // Cannot cast when Acceptable Losses is the only card in hand.
    let mut alone = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[fangren_hunter()],
        &[acceptable_losses()],
        &[],
    );
    tap_all_mana(&mut alone, P0);
    assert!(!lk::castable(&alone, P0, acceptable_losses()));
}
