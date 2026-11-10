//! Oracle scenarios for Aether Mutation.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

fn aether_mutation() -> CardIndex {
    card_index("6697fe5b-90ac-4321-aa2f-cdc6ec283cb4")
}

#[test]
fn aether_mutation_bounces_target_creature_and_creates_saprolings_equal_to_its_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), island()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[aether_mutation()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let giant = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    cast_from_hand(&mut e, p0, aether_mutation());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, p1, index::HILL_GIANT).is_none(),
        "Hill Giant left the battlefield"
    );
    assert!(
        in_hand(&e, p1, index::HILL_GIANT).is_some(),
        "Hill Giant returned to its owner's hand"
    );
    let saprolings: Vec<_> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == p0
                    && o.card.is_none()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SAPROLING)
            })
        })
        .collect();
    assert_eq!(
        saprolings.len(),
        4,
        "Hill Giant has mana value 4, so 4 Saproling tokens are created"
    );
    for id in saprolings {
        assert_eq!(pt(&e, id), (1, 1));
    }
}

#[test]
fn aether_mutation_targeting_a_zero_mana_value_creature_creates_zero_saprolings() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                index::ORNITHOPTER,
            ],
        )
        .hand(0, &[aether_mutation()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let thopter = on_battlefield(&e, p0, index::ORNITHOPTER).unwrap();
    cast_from_hand(&mut e, p0, aether_mutation());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![thopter],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_hand(&e, p0, index::ORNITHOPTER).is_some());
    let saprolings = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            e.state().object(**id).is_some_and(|o| {
                o.card.is_none()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SAPROLING)
            })
        })
        .count();
    assert_eq!(saprolings, 0, "mana value 0 creates 0 Saproling tokens");
}
