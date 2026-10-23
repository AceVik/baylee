//! `cards/instants/mv_1/mind_games.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_games() -> CardIndex {
    card_index("ea43372f-b0a7-4d1b-be3e-a4a738b06d59")
}

/// Mind Games (`ea43372f-b0a7-4d1b-be3e-a4a738b06d59`) — {U} Instant (Partial):
/// "Tap target artifact, creature, or land."
#[test]
fn mind_games_taps_target_artifact_creature_or_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[mind_games()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, mind_games());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, mind_games()).is_some(),
        "base spell does not get free buyback"
    );

    assert!(is_tapped(&engine, bear));
}

#[test]
fn artifacts_and_lands_are_legal_but_enchantments_are_not() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::SOL_RING, forest(), index::MEGRIM],
        &[mind_games(), mind_games()],
        &[],
    );
    lk::float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    let enchantment = lk::obj(&e, P1, index::MEGRIM);
    for target_card in [index::SOL_RING, forest()] {
        let target = lk::obj(&e, P1, target_card);
        cast_with_floating(&mut e, P0, mind_games());
        assert!(
            e.apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![enchantment],
                    players: vec![]
                }
            )
            .is_err()
        );
        lk::aim(&mut e, &[target], &[]);
        lk::settle(&mut e);
        assert!(lk::tapped(&e, target));
    }
}
