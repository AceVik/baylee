//! Oracle scenarios for Death Mutation.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

fn death_mutation() -> CardIndex {
    card_index("6f705338-b00b-4a6d-924f-443e7c5670b7")
}

#[test]
fn death_mutation_destroys_nonblack_creature_without_regeneration_and_creates_saprolings() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(
            1,
            &[index::HILL_GIANT, index::DRUDGE_SKELETONS, index::SWAMP],
        )
        .hand(0, &[death_mutation()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let giant = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    let skeletons = on_battlefield(&e, p1, index::DRUDGE_SKELETONS).unwrap();

    // Give Hill Giant a regeneration shield to verify "It can't be regenerated."
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(giant)
        .unwrap()
        .regeneration_shields = 1;

    cast_from_hand(&mut e, p0, death_mutation());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target prompt")
    };
    assert!(
        options.contains(&giant) && !options.contains(&skeletons),
        "only nonblack creatures can be targeted"
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, p1, index::HILL_GIANT).is_some(),
        "Hill Giant was destroyed despite a regeneration shield"
    );
    let saprolings = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            e.state().object(**id).is_some_and(|o| {
                o.controller == p0
                    && o.card.is_none()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SAPROLING)
            })
        })
        .count();
    assert_eq!(
        saprolings, 4,
        "Hill Giant has mana value 4, so 4 Saproling tokens are created"
    );
}
