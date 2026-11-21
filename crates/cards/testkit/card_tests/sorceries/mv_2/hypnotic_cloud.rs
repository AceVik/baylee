//! `cards/sorceries/mv_2/hypnotic_cloud.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hypnotic_cloud() -> CardIndex {
    card_index("277757c6-4c4c-4a4c-84f6-4a64fe3b3bfe")
}

fn cast_cloud(engine: &mut Engine<RegistryLookup>, kicked: bool, target: PlayerId) {
    cast_with_floating(engine, P0, hypnotic_cloud());
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::YesNo { .. } => lk::yes_no(engine, kicked),
            Pending::ChooseCastMode { .. } => {
                let kind = if kicked {
                    CastModeKind::Kicked
                } else {
                    CastModeKind::Normal
                };
                let slot = choose_cast_kind(engine, kind);
                engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChoosePlayer { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChoosePlayer(target))
                    .unwrap();
            }
            Pending::ChooseTargets { .. } => lk::aim(engine, &[], &[target]),
            _ => break,
        }
    }
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, target);
    let expected = if kicked { 3 } else { 1 };
    assert_eq!((min, max), (expected, expected));
    let chosen = options.into_iter().take(usize::from(expected)).collect();
    engine
        .apply(target, PlayerAction::ChooseObjects { objects: chosen })
        .unwrap();
    lk::settle(engine);
}

/// Hypnotic Cloud (277757c6-4c4c-4a4c-84f6-4a64fe3b3bfe) — {1}{B} Sorcery,
/// Kicker {4}: "Target player discards a card. If this spell was kicked, that
/// player discards three cards instead."
#[test]
fn hypnotic_cloud_discards_one_unkicked_and_three_instead_when_kicked() {
    let mut unkicked = lk::game(
        SEED,
        &[swamp(); 6],
        &[],
        &[hypnotic_cloud()],
        &[forest(), island(), plains(), mountain()],
    );
    tap_all_mana(&mut unkicked, P0);
    cast_cloud(&mut unkicked, false, P1);
    assert_eq!(lk::pool_total(&unkicked, P0), 4, "only {{1}}{{B}} spent");
    assert_eq!(
        unkicked.state().zones.list(ZoneLocation::Hand(P1)).len(),
        3,
        "discarded 1 card"
    );

    let mut kicked = lk::game(
        SEED,
        &[swamp(); 6],
        &[],
        &[hypnotic_cloud()],
        &[forest(), island(), plains(), mountain()],
    );
    tap_all_mana(&mut kicked, P0);
    cast_cloud(&mut kicked, true, P1);
    assert_eq!(lk::pool_total(&kicked, P0), 0, "all {{5}}{{B}} spent");
    assert_eq!(
        kicked.state().zones.list(ZoneLocation::Hand(P1)).len(),
        1,
        "discarded 3 cards instead of 1"
    );
}
