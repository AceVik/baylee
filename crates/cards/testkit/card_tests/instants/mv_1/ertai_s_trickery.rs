//! `cards/instants/mv_1/ertai_s_trickery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "93d0691f-17b8-458b-a624-e182fc481f49"
fn ertai_s_trickery() -> CardIndex {
    card_index("93d0691f-17b8-458b-a624-e182fc481f49")
}

/// Ertai's Trickery ({U}): counters a kicked spell on the stack.
#[test]
fn ertais_trickery_counters_a_kicked_spell() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[ertai_s_trickery()])
        .battlefield(
            1,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(1, &[index::EXPLOSIVE_GROWTH])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::EXPLOSIVE_GROWTH);
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    let growth = on_stack(&engine, index::EXPLOSIVE_GROWTH).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ertai_s_trickery());
    lk::aim(&mut engine, &[growth], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (2, 2),
        "kicked Explosive Growth was countered and never pumped Bears"
    );
    assert!(in_graveyard(&engine, P1, index::EXPLOSIVE_GROWTH).is_some());
}

/// Ertai's Trickery can legally target an unkicked spell, but does NOT counter
/// it when it resolves.
#[test]
fn ertais_trickery_does_not_counter_an_unkicked_spell() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[ertai_s_trickery()])
        .battlefield(1, &[forest(), index::GRIZZLY_BEARS])
        .hand(1, &[index::EXPLOSIVE_GROWTH])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::EXPLOSIVE_GROWTH);
    lk::aim(&mut engine, &[bears], &[]);
    let growth = on_stack(&engine, index::EXPLOSIVE_GROWTH).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ertai_s_trickery());
    lk::aim(&mut engine, &[growth], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (4, 4),
        "unkicked Explosive Growth was NOT countered and gave +2/+2"
    );
    assert!(in_graveyard(&engine, P0, ertai_s_trickery()).is_some());
}
