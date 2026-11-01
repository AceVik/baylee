//! Oracle scenarios for Aven Fisher.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn aven_fisher() -> CardIndex {
    card_index("69a72414-d02f-4a5c-b5c9-8941bb181f61")
}

#[test]
fn aven_fisher_dies_trigger_may_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::MOUNTAIN,
            ],
        )
        .hand(0, &[aven_fisher(), index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, aven_fisher());
    pass_until(&mut e, stack_is_empty);

    let fisher = on_battlefield(&e, p0, aven_fisher()).unwrap();
    assert!(keywords(&e, fisher).contains(KeywordSet::FLYING));
    let lib_before = library_size(&e, p0);

    cast_with_floating(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![fisher],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, aven_fisher()).is_some());
    assert_eq!(library_size(&e, p0), lib_before - 1);
}

#[test]
fn aven_fisher_dies_trigger_can_decline_draw() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::MOUNTAIN,
            ],
        )
        .hand(0, &[aven_fisher(), index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, aven_fisher());
    pass_until(&mut e, stack_is_empty);

    let fisher = on_battlefield(&e, p0, aven_fisher()).unwrap();
    let lib_before = library_size(&e, p0);

    cast_with_floating(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![fisher],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, aven_fisher()).is_some());
    assert_eq!(library_size(&e, p0), lib_before);
}
