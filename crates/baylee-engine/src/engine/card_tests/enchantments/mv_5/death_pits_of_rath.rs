//! `cards/enchantments/mv_5/death_pits_of_rath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn death_pits_of_rath() -> CardIndex {
    card_index("aead53ef-ffcf-4ae2-ab81-40aa9627adc9")
}

/// Death Pits of Rath (`aead53ef-ffcf-4ae2-ab81-40aa9627adc9`) — {3}{B}{B} Enchantment (Partial):
/// "Whenever a creature is dealt damage, destroy it. It can't be regenerated."
#[test]
fn death_pits_of_rath_destroys_a_creature_dealt_nonlethal_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[death_pits_of_rath(), index::PRODIGAL_SORCERER])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    activate(&mut engine, P0, index::PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, P0, index::PRODIGAL_SORCERER).is_some());
}

#[test]
fn death_pits_of_rath_does_not_destroy_an_earlier_damaged_creature_again() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[
            index::PRODIGAL_SORCERER,
            index::PRODIGAL_SORCERER,
            index::SERRA_ANGEL,
        ],
        &[index::HILL_GIANT],
        &[death_pits_of_rath()],
        &[],
    );
    let angel = lk::obj(&e, P0, index::SERRA_ANGEL);
    let giant = lk::obj(&e, P1, index::HILL_GIANT);
    let sorcerers: Vec<_> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .unwrap()
                .card
                .is_some_and(|card| card.index == index::PRODIGAL_SORCERER)
        })
        .collect();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: sorcerers[0],
            ability_index: 0,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[angel], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, angel), 1);
    lk::float(&mut e, P0, &[(ManaColor::Black, 5)]);
    lk::cast_at(&mut e, P0, death_pits_of_rath(), &[], &[]);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: sorcerers[1],
            ability_index: 0,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[giant], &[]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&e, P0, index::SERRA_ANGEL).is_some());
}
