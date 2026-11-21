//! `cards/instants/mv_5/submerge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "99427ebe-c00d-4206-84ca-9764f6e952c6"
fn submerge() -> CardIndex {
    card_index("99427ebe-c00d-4206-84ca-9764f6e952c6")
}

/// Submerge ({4}{U}): free cost requires BOTH own Island AND opponent's Forest;
/// when met, casts for 0 mana and puts target creature on top of its owner's
/// library so its owner draws it next turn.
#[test]
fn submerge_free_cost_requires_island_and_opponent_forest_and_tops_library() {
    let only_island = table(SEED, &[submerge()], &[island()], &[], &[index::HILL_GIANT]);
    assert!(
        !lk::castable(&only_island, me(), submerge()),
        "Island alone is insufficient without opponent's Forest"
    );

    let only_forest = table(
        SEED,
        &[submerge()],
        &[],
        &[],
        &[forest(), index::HILL_GIANT],
    );
    assert!(
        !lk::castable(&only_forest, me(), submerge()),
        "opponent's Forest alone is insufficient without own Island"
    );

    let mut engine = table(
        SEED,
        &[submerge()],
        &[island()],
        &[],
        &[forest(), index::HILL_GIANT],
    );
    let giant = on_battlefield(&engine, them(), index::HILL_GIANT).unwrap();
    let lib_before = library_len(&engine, them());
    let spell = in_hand(&engine, me(), submerge()).unwrap();

    engine
        .apply(me(), PlayerAction::CastSpell { card: spell })
        .expect("Island + opponent's Forest makes Submerge free");
    aim_at(&mut engine, me(), giant);
    settle(&mut engine);

    assert!(on_battlefield(&engine, them(), index::HILL_GIANT).is_none());
    assert_eq!(library_len(&engine, them()), lib_before + 1);
    let top_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(them()))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top_id).unwrap().card.unwrap().index,
        index::HILL_GIANT,
        "Hill Giant is on TOP of its owner's library"
    );

    // On P1's next turn, P1 draws Hill Giant straight into hand.
    reach_their_main_phase(&mut engine, them());
    assert!(
        in_hand(&engine, them(), index::HILL_GIANT).is_some(),
        "owner drew the submerged creature on their draw step"
    );
}

/// Without opponent's Forest, Submerge casts normally for {4}{U} and puts
/// target creature on top of its owner's library.
#[test]
fn submerge_casts_for_five_mana_when_free_condition_unmet() {
    let mut engine = table(
        SEED,
        &[submerge()],
        &[island(), island(), island(), island(), island()],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let bears = on_battlefield(&engine, them(), index::GRIZZLY_BEARS).unwrap();

    cast_at(&mut engine, submerge(), bears);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(on_battlefield(&engine, them(), index::GRIZZLY_BEARS).is_none());
    let top_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(them()))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top_id).unwrap().card.unwrap().index,
        index::GRIZZLY_BEARS,
        "Grizzly Bears put on top of owner's library"
    );
}
