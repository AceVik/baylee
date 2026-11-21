//! `cards/sorceries/mv_3/bog_down.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bog_down() -> CardIndex {
    card_index("8574f57a-768d-42a0-ac8c-cc0a1de181e0")
}

/// Bog Down (8574f57a-768d-42a0-ac8c-cc0a1de181e0) — {2}{B} Sorcery:
/// "Target player discards two cards." when unkicked.
#[test]
fn bog_down_discards_two_when_unkicked() {
    let mut unkicked = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[],
        &[bog_down()],
        &[forest(), island(), plains(), mountain()],
    );
    tap_all_mana(&mut unkicked, P0);
    cast_with_floating(&mut unkicked, P0, bog_down());
    for _ in 0..6 {
        match unkicked.pending().clone() {
            Pending::YesNo { .. } => lk::yes_no(&mut unkicked, false),
            Pending::ChooseCastMode { .. } => {
                let slot = choose_cast_kind(&unkicked, CastModeKind::Normal);
                unkicked.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChoosePlayer { player, .. } => {
                unkicked
                    .apply(player, PlayerAction::ChoosePlayer(P1))
                    .unwrap();
            }
            Pending::ChooseTargets { .. } => lk::aim(&mut unkicked, &[], &[P1]),
            _ => break,
        }
    }
    pass_until(&mut unkicked, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = unkicked.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((min, max), (2, 2));
    unkicked
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0], options[1]],
            },
        )
        .unwrap();
    lk::settle(&mut unkicked);
    assert_eq!(unkicked.state().zones.list(ZoneLocation::Hand(P1)).len(), 2);
}

/// Bog Down — Kicker—Sacrifice two lands: "If this spell was kicked, that
/// player discards three cards instead."
#[test]
fn bog_down_sacrifices_two_lands_for_kicker_to_discard_three_instead() {
    let mut kicked = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[],
        &[bog_down()],
        &[forest(), island(), plains(), mountain()],
    );
    tap_all_mana(&mut kicked, P0);
    cast_with_floating(&mut kicked, P0, bog_down());
    for _ in 0..8 {
        match kicked.pending().clone() {
            Pending::YesNo { .. } => lk::yes_no(&mut kicked, true),
            Pending::ChooseCastMode { .. } => {
                let slot = choose_cast_kind(&kicked, CastModeKind::Kicked);
                kicked.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChoosePlayer { player, .. } => {
                kicked
                    .apply(player, PlayerAction::ChoosePlayer(P1))
                    .unwrap();
            }
            Pending::ChooseTargets { .. } => lk::aim(&mut kicked, &[], &[P1]),
            Pending::ChooseCards {
                player, options, ..
            } => {
                kicked
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    assert_eq!(
        kicked.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2,
        "two lands sacrificed as kicker cost"
    );
    pass_until(&mut kicked, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options: k_opts,
        min: k_min,
        max: k_max,
        ..
    } = kicked.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((k_min, k_max), (3, 3));
    kicked
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![k_opts[0], k_opts[1], k_opts[2]],
            },
        )
        .unwrap();
    lk::settle(&mut kicked);
    assert_eq!(kicked.state().zones.list(ZoneLocation::Hand(P1)).len(), 1);
}
