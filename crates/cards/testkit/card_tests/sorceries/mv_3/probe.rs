//! `cards/sorceries/mv_3/probe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn probe() -> CardIndex {
    card_index("f8d227b6-1627-4ea1-b815-887094497abf")
}

fn settle_discards(engine: &mut Engine<RegistryLookup>) -> Vec<(PlayerId, u8)> {
    let mut asked = Vec::new();
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { .. } if stack_is_empty(engine) => return asked,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(min, max);
                asked.push((player, min));
                let chosen = options.into_iter().take(usize::from(min)).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: chosen })
                    .unwrap();
            }
            other => panic!("unexpected pending during Probe: {other:?}"),
        }
    }
    panic!("Probe never settled");
}

/// Probe (f8d227b6-1627-4ea1-b815-887094497abf) — {2}{U} Sorcery, Kicker {1}{B}:
/// "Draw three cards, then discard two cards. If this spell was kicked, target
/// player discards two cards."
#[test]
fn probe_unkicked_loots_without_target_and_kicked_also_makes_target_discard_two() {
    let mut unkicked = lk::game(
        SEED,
        &[island(), island(), island(), swamp(), swamp()],
        &[],
        &[probe()],
        &[forest(), island(), plains()],
    );
    let p0_lib = library_size(&unkicked, P0);
    tap_all_mana(&mut unkicked, P0);
    cast_with_floating(&mut unkicked, P0, probe());
    if matches!(unkicked.pending(), Pending::ChooseCastMode { .. }) {
        let normal = choose_cast_kind(&unkicked, CastModeKind::Normal);
        unkicked
            .apply(P0, PlayerAction::ChooseMode(normal))
            .unwrap();
    } else if matches!(unkicked.pending(), Pending::YesNo { .. }) {
        lk::yes_no(&mut unkicked, false);
    }
    let discards = settle_discards(&mut unkicked);
    assert_eq!(discards, vec![(P0, 2)], "unkicked asks no target discard");
    assert_eq!(library_size(&unkicked, P0), p0_lib - 3);
    assert_eq!(unkicked.state().zones.list(ZoneLocation::Hand(P0)).len(), 1);
    assert_eq!(unkicked.state().zones.list(ZoneLocation::Hand(P1)).len(), 3);

    let mut kicked = lk::game(
        SEED,
        &[island(), island(), island(), swamp(), swamp()],
        &[],
        &[probe()],
        &[forest(), island(), plains()],
    );
    tap_all_mana(&mut kicked, P0);
    cast_with_floating(&mut kicked, P0, probe());
    for _ in 0..6 {
        match kicked.pending().clone() {
            Pending::ChooseCastMode { .. } => {
                let slot = choose_cast_kind(&kicked, CastModeKind::Kicked);
                kicked.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::YesNo { .. } => lk::yes_no(&mut kicked, true),
            Pending::ChoosePlayer { player, .. } => {
                kicked
                    .apply(player, PlayerAction::ChoosePlayer(P1))
                    .unwrap();
            }
            Pending::ChooseTargets { .. } => lk::aim(&mut kicked, &[], &[P1]),
            _ => break,
        }
    }
    assert_eq!(lk::pool_total(&kicked, P0), 0, "all {{3}}{{U}}{{B}} spent");
    let kicked_discards = settle_discards(&mut kicked);
    assert_eq!(kicked_discards, vec![(P0, 2), (P1, 2)]);
    assert_eq!(kicked.state().zones.list(ZoneLocation::Hand(P1)).len(), 1);
}
