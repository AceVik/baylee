//! `cards/instants/mv_1/run_wild.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Run Wild (`91d69d6a-3832-4d20-a02b-1e83c08ac723`) — {G} Instant:
/// "Until end of turn, target creature gains trample and \"{G}: Regenerate this creature.\""
#[test]
fn run_wild_grants_trample_and_regenerate_activated_ability() {
    let run_wild_card = card_index("91d69d6a-3832-4d20-a02b-1e83c08ac723");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[run_wild_card])
        .battlefield(0, &[index::GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    assert!(!keywords(&engine, bears).contains(KeywordSet::TRAMPLE));

    cast_from_hand(&mut engine, p0, run_wild_card);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::TRAMPLE));

    // Activate the granted `{G}: Regenerate this creature` ability and verify the regeneration shield saves Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bears,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    bury(&mut engine, &[bears]);
    assert!(
        on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears regenerated instead of dying"
    );
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
