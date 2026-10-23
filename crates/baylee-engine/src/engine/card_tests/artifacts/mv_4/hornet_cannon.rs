//! `cards/artifacts/mv_4/hornet_cannon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn hornet_cannon() -> CardIndex {
    card_index("595ae6f1-e66a-4774-943a-9d5368f4a3b4")
}

/// Hornet Cannon (`595ae6f1-e66a-4774-943a-9d5368f4a3b4`) — {4} Artifact (Partial):
/// "{3}, {T}: Create a 1/1 colorless Insect artifact creature token with flying and haste named Hornet."
#[test]
fn hornet_cannon_creates_flying_haste_hornet_artifact_creature_token() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hornet_cannon(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, hornet_cannon(), 0);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(power_toughness(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::HASTE));
}
