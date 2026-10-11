//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn matrix_grants_a_persistent_regeneration_ability_and_charges_the_actual_counter() {
    use baylee_cards_dsl::counters;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::LIFE_MATRIX,
                index::GRIZZLY_BEARS,
                index::DOUBLING_SEASON,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(),Pending::Priority{player,..} if *player==p0)
    });
    let matrix = on_battlefield(&e, p0, index::LIFE_MATRIX).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::LIFE_MATRIX, 0);
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, counters::MATRIX), 2);
    bury(&mut e, &[matrix]);
    assert!(walk_to_own_main(&mut e, p0));
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: bear,
            ability_index: crate::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, counters::MATRIX), 1);
    assert_eq!(e.state().object(bear).unwrap().regeneration_shields, 1);
    bury(&mut e, &[bear]);
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(bear).unwrap().regeneration_shields, 0);
    reach_their_main_phase(&mut e, PlayerId::new(1));
    assert!(walk_to_own_main(&mut e, p0));
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: bear,
            ability_index: crate::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, counters::MATRIX), 0);
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: crate::choice::GRANTED_ABILITY
            }
        )
        .is_err()
    );
}
