//! `cards/artifacts/mv_5/cauldron_of_souls.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn cauldron_of_souls() -> CardIndex {
    card_index("5811d905-cae5-4f92-a78e-abb43f8864a7")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Cauldron of Souls (5811d905-cae5-4f92-a78e-abb43f8864a7) — {5} Artifact:
/// "{T}: Choose any number of target creatures. Each of those creatures gains
/// persist until end of turn."
///
/// Grants persist to multiple targeted creatures at once: both return with a
/// -1/-1 counter when killed, and dying a second time while carrying a -1/-1
/// counter keeps the creature in the graveyard.
#[test]
fn cauldron_of_souls_grants_persist_to_multiple_targets_and_blocks_second_return() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                cauldron_of_souls(),
                grizzly_bears(),
                hill_giant(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cauldron = on_battlefield(&engine, p0, cauldron_of_souls()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();

    // Activate Cauldron targeting BOTH Grizzly Bears and Hill Giant.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cauldron,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears, giant],
            },
        )
        .expect("choose two target creatures");
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::PERSIST));
    assert!(keywords(&engine, giant).contains(KeywordSet::PERSIST));

    // Kill both creatures with Lightning Bolts: both persist back with a -1/-1 counter.
    tap_all_mana(&mut engine, p0);
    for target in [bears, giant] {
        let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
        engine
            .apply(p0, PlayerAction::CastSpell { card: bolt })
            .unwrap();
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }

    let returned_bears =
        on_battlefield(&engine, p0, grizzly_bears()).expect("Bears persisted back");
    let returned_giant =
        on_battlefield(&engine, p0, hill_giant()).expect("Hill Giant persisted back");
    assert_eq!(counters_on(&engine, returned_bears, CounterKind::M1M1), 1);
    assert_eq!(pt(&engine, returned_bears), (1, 1));
    assert_eq!(counters_on(&engine, returned_giant, CounterKind::M1M1), 1);
    assert_eq!(pt(&engine, returned_giant), (2, 2));

    // Kill returned Bears a second time: it has a -1/-1 counter (and is a new object),
    // so it stays in the graveyard.
    let bolt3 = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt3 })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![returned_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "creature with a -1/-1 counter does not persist again"
    );
}

/// Supports 0 targets, expires at end of turn, and returns an opponent's
/// creature under its owner's control.
#[test]
fn cauldron_of_souls_zero_targets_expiry_and_opponent_owner_return() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cauldron_of_souls(), grizzly_bears(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cauldron = on_battlefield(&engine, p0, cauldron_of_souls()).unwrap();
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let p0_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // Target opponent's Grizzly Bears and kill it -> returns under p1's (owner's) control.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cauldron,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p1_bears, p0_bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let p1_back = on_battlefield(&engine, p1, grizzly_bears())
        .expect("opponent's creature returned under its owner's control");
    assert_eq!(counters_on(&engine, p1_back, CounterKind::M1M1), 1);

    // Advance to next p0 turn: persist on p0's Bears has expired, and 0 targets is legal.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, p0_bears).contains(KeywordSet::PERSIST),
        "persist expired at end of turn"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cauldron,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("choosing 0 targets is legal ('any number')");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, cauldron));
}
