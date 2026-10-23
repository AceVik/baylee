//! `cards/creatures/mv_2/llanowar_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn llanowar_druid() -> CardIndex {
    card_index("b8cadeee-36b7-4a61-91ff-7d6c7c780464")
}

fn tap(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    e.state
        .object_mut(id)
        .expect("object exists")
        .status
        .insert(Status::TAPPED);
}

/// Llanowar Druid (b8cadeee-36b7-4a61-91ff-7d6c7c780464) — "{T}, Sacrifice
/// this creature: Untap all Forests." All Forests, whoever controls them: the
/// caster's two and the opponent's one come untapped; the Mountain and the
/// Plains, which are not Forests, stay tapped; the Druid is sacrificed.
#[test]
fn llanowar_druid_untaps_every_forest_and_nothing_else() {
    let mut e = lk::game(
        SEED,
        &[llanowar_druid(), forest(), forest(), mountain()],
        &[forest(), plains()],
        &[],
        &[],
    );
    let theirs_forest = lk::obj(&e, P1, forest());
    let theirs_plains = lk::obj(&e, P1, plains());
    let mountain_id = lk::obj(&e, P0, mountain());
    tap_all_mana(&mut e, P0);
    tap(&mut e, theirs_forest);
    tap(&mut e, theirs_plains);
    assert!(
        lk::tapped(&e, mountain_id) && lk::tapped(&e, theirs_forest),
        "all tapped first"
    );

    lk::use_ability(&mut e, P0, llanowar_druid(), 0, &[], &[]);

    assert!(
        in_graveyard(&e, P0, llanowar_druid()).is_some(),
        "sacrificed as a cost"
    );
    let forests: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 3, "two of ours and one of theirs");
    for forest_id in forests {
        assert!(
            !lk::tapped(&e, forest_id),
            "every Forest untaps: {forest_id:?}"
        );
    }
    assert!(lk::tapped(&e, mountain_id), "a Mountain is not a Forest");
    assert!(lk::tapped(&e, theirs_plains), "a Plains is not a Forest");
}

/// Llanowar Druid — "{T}" in the cost: cast this turn it is summoning sick and
/// the ability is not offered.
#[test]
fn llanowar_druid_cannot_be_used_the_turn_it_comes_in() {
    let mut e = lk::game(SEED, &[forest(), forest()], &[], &[llanowar_druid()], &[]);

    cast_from_hand(&mut e, P0, llanowar_druid());
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, llanowar_druid()).is_some());
    assert!(
        !lk::offered(&e, llanowar_druid(), P0, 0),
        "a creature that has not been under your control since your turn began cannot {{T}}"
    );
}
