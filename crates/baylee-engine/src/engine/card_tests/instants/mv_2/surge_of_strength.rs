//! `cards/instants/mv_2/surge_of_strength.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn surge_of_strength() -> CardIndex {
    card_index("bd3e55cf-ffca-402c-91ea-2420035a9dd1")
}

/// Surge of Strength ({R}{G} Instant):
/// - "As an additional cost to cast this spell, discard a red or green card."
/// - "Target creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value."
#[test]
fn surge_of_strength_discards_red_or_green_card_and_pumps_by_target_mana_value_with_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[HILL_GIANT, mountain(), forest()])
        .hand(0, &[surge_of_strength(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let surge = in_hand(&engine, p0, surge_of_strength()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: surge })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    // Hill Giant ({3}{R}, mana value 4, 3/3) gets +4/+0 and trample -> 7/3 with trample.
    assert_eq!(pt(&engine, giant), (7, 3));
    assert!(keywords(&engine, giant).contains(KeywordSet::TRAMPLE));
}

/// "discard a red or green card": a blue card is not one, so with nothing else
/// in hand the spell cannot be cast, and with a green card beside it only the
/// green card is offered.
#[test]
fn surge_of_strength_discards_only_a_red_or_green_card() {
    use baylee_core::generated::index::AIR_ELEMENTAL;
    let p0 = PlayerId::new(0);

    // Only a blue card to discard: not castable.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[HILL_GIANT, mountain(), forest()])
        .hand(0, &[surge_of_strength(), AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let surge = in_hand(&engine, p0, surge_of_strength()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&surge),
        "no red or green card to discard"
    );

    // A green card and a blue card: only the green one is offered.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[HILL_GIANT, mountain(), forest()])
        .hand(0, &[surge_of_strength(), grizzly_bears(), AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let blue = in_hand(&engine, p0, AIR_ELEMENTAL).unwrap();
    let surge = in_hand(&engine, p0, surge_of_strength()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: surge })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the cost asks which card to discard, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&bears), "a green card: {options:?}");
    assert!(!options.contains(&blue), "a blue card: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, AIR_ELEMENTAL).is_some());
}

/// "where X is that creature's mana value": any creature is a target, and X
/// is its own mana value (Grizzly Bears, {1}{G}: +2/+0).
#[test]
fn surge_of_strength_adds_the_targets_own_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[surge_of_strength(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let discard = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let surge = in_hand(&engine, p0, surge_of_strength()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: surge })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, theirs), (4, 2));
    assert!(keywords(&engine, theirs).contains(KeywordSet::TRAMPLE));
}
