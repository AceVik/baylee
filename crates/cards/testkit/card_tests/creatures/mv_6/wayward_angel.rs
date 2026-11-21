//! `cards/creatures/mv_6/wayward_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wayward_angel() -> CardIndex {
    card_index("bf169a7a-e3cf-44b2-83eb-c2597bcf22d9")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Wayward Angel (bf169a7a-e3cf-44b2-83eb-c2597bcf22d9) — `{4}{W}{W}` 4/4
/// Creature — Angel Horror:
/// - Flying, vigilance
/// - Threshold — As long as there are seven or more cards in your graveyard, this
///   creature gets +3/+3, is black, has trample, and has "At the beginning of your
///   upkeep, sacrifice a creature."
#[test]
fn wayward_angel_transforms_stats_color_trample_and_upkeep_sacrifice_at_threshold() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[wayward_angel(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    seed_graveyard(&mut engine, P0, 6);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(mountain()));
    cast_with_floating(&mut engine, P0, wayward_angel());
    pass_until(&mut engine, stack_is_empty);

    // At 6 cards in graveyard: 4/4 White, Flying + Vigilance, no Trample.
    let angel = on_battlefield(&engine, P0, wayward_angel()).expect("Wayward Angel entered");
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, angel), (4, 4));
    assert_eq!(
        engine
            .state()
            .object(angel)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::White)
    );
    let kw6 = keywords(&engine, angel);
    assert!(kw6.contains(KeywordSet::FLYING) && kw6.contains(KeywordSet::VIGILANCE));
    assert!(!kw6.contains(KeywordSet::TRAMPLE));

    // Cast Lightning Bolt at P1 -> 7th card enters P0's graveyard -> Threshold turns on!
    cast_from_hand(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, angel), (7, 7));
    assert_eq!(
        engine
            .state()
            .object(angel)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Black),
        "`is black` overwrites White"
    );
    assert!(keywords(&engine, angel).contains(KeywordSet::TRAMPLE));

    // Next P0 upkeep: granted trigger forces sacrificing a creature.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, P0);
    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, P0, wayward_angel()).is_some());
}
