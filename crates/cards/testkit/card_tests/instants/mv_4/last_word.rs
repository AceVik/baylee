//! `cards/instants/mv_4/last_word.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "33d6a17a-0112-4d4f-a24a-0769360408a8"
fn last_word() -> CardIndex {
    card_index("33d6a17a-0112-4d4f-a24a-0769360408a8")
}

/// Last Word ({2}{U}{U}): can't be countered (survives Counterspell aimed at
/// it) and counters target spell.
#[test]
fn last_word_cannot_be_countered_and_counters_target_spell() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[last_word()])
        .battlefield(1, &[mountain(), island(), island()])
        .hand(1, &[lightning_bolt(), index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    // P1 casts Lightning Bolt at P0.
    let mtn = lk::obj(&engine, P1, mountain());
    tap_mana_where(&mut engine, P1, |id| id == mtn);
    cast_with_floating(&mut engine, P1, lightning_bolt());
    lk::aim(&mut engine, &[], &[P0]);
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    // P0 casts Last Word targeting Lightning Bolt.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, last_word());
    lk::aim(&mut engine, &[bolt], &[]);
    let lw = on_stack(&engine, last_word()).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 tries to Counterspell Last Word -> Counterspell resolves, Last Word
    // stays on stack and still counters Lightning Bolt.
    let islands = all_on_battlefield(&engine, P1, island());
    tap_mana_where(&mut engine, P1, |id| islands.contains(&id));
    cast_with_floating(&mut engine, P1, index::COUNTERSPELL);
    lk::aim(&mut engine, &[lw], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        lk::life(&engine, P0),
        20,
        "Last Word survived Counterspell and countered Bolt"
    );
    assert!(in_graveyard(&engine, P1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, P1, index::COUNTERSPELL).is_some());
    assert!(in_graveyard(&engine, P0, last_word()).is_some());
}
