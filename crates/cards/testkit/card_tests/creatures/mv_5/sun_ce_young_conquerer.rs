//! `cards/creatures/mv_5/sun_ce_young_conquerer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sun_ce_young_conquerer() -> CardIndex {
    card_index("93b0185c-b86f-4137-bc6d-96212eee1708")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Sun Ce, Young Conquerer (93b0185c-b86f-4137-bc6d-96212eee1708) — `{3}{U}{U}` 3/3
/// Legendary Creature — Human Soldier:
/// - Horsemanship
/// - When this creature enters, you may return target creature to its owner's hand.
#[test]
fn sun_ce_young_conquerer_may_bounce_target_creature_on_etb_and_has_horsemanship() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[sun_ce_young_conquerer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, sun_ce_young_conquerer());
    let options = pass_until_targets(&mut engine, P0);
    let sun_ce = on_battlefield(&engine, P0, sun_ce_young_conquerer()).expect("Sun Ce entered");
    assert!(options.contains(&bears) && options.contains(&sun_ce));

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, sun_ce), (3, 3));
    assert!(keywords(&engine, sun_ce).contains(KeywordSet::HORSEMANSHIP));
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_none());
    assert!(in_hand(&engine, P1, grizzly_bears()).is_some());
}

#[test]
fn sun_ce_young_conquerer_allows_declining_the_optional_bounce() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[sun_ce_young_conquerer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, sun_ce_young_conquerer());
    let _ = pass_until_targets(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
}
