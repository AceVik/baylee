//! `cards/artifacts/mv_2/excavator.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn excavator() -> CardIndex {
    card_index("2d5ebcd7-07c2-412e-b0d9-54e3a88895fd")
}

/// Excavator (`2d5ebcd7-07c2-412e-b0d9-54e3a88895fd`) — {2} Artifact (Partial):
/// "{T}, Sacrifice a basic land: Target creature gains landwalk of each of the land types of the sacrificed land until end of turn."
#[test]
fn excavator_sacrifices_basic_land_to_grant_landwalk_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[excavator(), forest(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let exc = on_battlefield(&engine, P0, excavator()).unwrap();
    let land = on_battlefield(&engine, P0, forest()).unwrap();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: exc,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, P0, bears);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, forest()).is_none());
    assert!(keywords(&engine, bears).contains(KeywordSet::FORESTWALK));

    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, bears).contains(KeywordSet::FORESTWALK));
}
