//! Insidious Bookworms — `{B}` 1/1 Creature — Worm (Alliances)
//!
//! When this creature dies, you may pay `{1}{B}`. If you do, target player
//! discards a card at random.

#[allow(clippy::wildcard_imports)]
use super::*;

fn insidious_bookworms() -> CardIndex {
    card_index("ff3eeed9-6801-4cab-a97a-72f5f28acb24")
}

#[test]
fn insidious_bookworms_dies_trigger_pays_mana_to_make_target_player_discard_at_random() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[insidious_bookworms(), mountain(), swamp(), swamp()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let worms = on_battlefield(&engine, p0, insidious_bookworms()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![worms],
                players: vec![],
            },
        )
        .unwrap();

    // Wait for the dies trigger target choice.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    // Accept the may-pay-{1}{B} prompt on resolution.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before - 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        1
    );
}

/// "You may pay {1}{B}": declining makes nobody discard, though the trigger had a target.
#[test]
fn insidious_bookworms_discards_nothing_when_the_mana_is_declined() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[insidious_bookworms(), mountain(), swamp(), swamp()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let worms = on_battlefield(&engine, p0, insidious_bookworms()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![worms],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, insidious_bookworms()).is_some(),
        "it died"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty()
    );
}
