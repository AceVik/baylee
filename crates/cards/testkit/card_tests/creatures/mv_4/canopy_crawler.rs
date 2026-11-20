//! `cards/creatures/mv_4/canopy_crawler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;
use baylee_engine::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);

fn canopy_crawler() -> CardIndex {
    card_index("7fc60ae2-75fc-49e1-9d56-aa5a43844210")
}

/// Canopy Crawler ({3}{G}, 2/2 Beast):
/// Amplify 1; {T}: Target creature gets +1/+1 until end of turn for each
/// +1/+1 counter on this creature.
#[test]
fn canopy_crawler_amplifies_beasts_and_pumps_equal_to_counters_until_eot() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), savannah_lions()])
        .hand(
            0,
            &[
                canopy_crawler(),
                index::GARRUK_S_COMPANION,
                index::GARRUK_S_COMPANION,
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, canopy_crawler());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let bear = in_hand(&engine, P0, grizzly_bears()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2, "only the two Beasts in hand are eligible");
    assert!(!options.contains(&bear));
    engine.apply(P0, PlayerAction::ChooseObjects { objects: options }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let crawler = on_battlefield(&engine, P0, canopy_crawler()).unwrap();
    assert_eq!(counters_on(&engine, crawler, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, crawler), (4, 4));

    // Next turn, Canopy Crawler loses summoning sickness and pumps Savannah Lions +2/+2 until EOT.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    let lions = on_battlefield(&engine, P0, savannah_lions()).unwrap();
    assert_eq!(pt(&engine, lions), (2, 1));
    activate(&mut engine, P0, canopy_crawler(), 0);
    aim_at(&mut engine, P0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, lions), (4, 3));

    // Cleanup wears off at end of turn.
    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1)
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, lions), (2, 1));
}

#[test]
fn canopy_crawler_zero_reveals_is_two_two_and_pumps_by_zero() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), grizzly_bears()])
        .hand(0, &[canopy_crawler(), index::GARRUK_S_COMPANION])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, canopy_crawler());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::ChooseObjects { objects: vec![] }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let crawler = on_battlefield(&engine, P0, canopy_crawler()).unwrap();
    assert_eq!(counters_on(&engine, crawler, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, crawler), (2, 2));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    activate(&mut engine, P0, canopy_crawler(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (2, 2));
}
