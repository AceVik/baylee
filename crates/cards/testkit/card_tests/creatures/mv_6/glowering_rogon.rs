//! `cards/creatures/mv_6/glowering_rogon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;
use baylee_engine::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);

fn glowering_rogon() -> CardIndex {
    card_index("e9ed7322-53d4-419a-809f-78824aeaf99e")
}

/// Glowering Rogon ({5}{G}, 4/4 Beast):
/// Amplify 1 (including Doubling Season replacement interaction CR 614.16).
#[test]
fn glowering_rogon_amplifies_beasts_and_scales_with_doubling_season() {
    // Plain Amplify 1 with 2 Beasts and 1 non-Beast in hand.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 6])
        .hand(
            0,
            &[
                glowering_rogon(),
                index::GARRUK_S_COMPANION,
                index::RAMPAGING_BALOTHS,
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, glowering_rogon());
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
    assert_eq!(options.len(), 2);
    assert!(!options.contains(&bear));
    engine.apply(P0, PlayerAction::ChooseObjects { objects: options }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let rogon = on_battlefield(&engine, P0, glowering_rogon()).unwrap();
    assert_eq!(counters_on(&engine, rogon, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, rogon), (6, 6));

    // With Doubling Season on battlefield, revealing 1 Beast places 2 +1/+1 counters.
    let mut doubled = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest(), forest(), doubling_season()])
        .hand(0, &[glowering_rogon(), index::GARRUK_S_COMPANION])
        .start();
    keep_mulligans(&mut doubled);
    reach_main_phase(&mut doubled, P0);

    cast_from_hand(&mut doubled, P0, glowering_rogon());
    pass_until(&mut doubled, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = doubled.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 1);
    doubled.apply(P0, PlayerAction::ChooseObjects { objects: options }).unwrap();
    pass_until(&mut doubled, stack_is_empty);

    let rogon_d = on_battlefield(&doubled, P0, glowering_rogon()).unwrap();
    assert_eq!(counters_on(&doubled, rogon_d, CounterKind::P1P1), 2);
    assert_eq!(pt(&doubled, rogon_d), (6, 6));
}

#[test]
fn glowering_rogon_zero_reveals_is_four_four() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 6])
        .hand(0, &[glowering_rogon(), index::GARRUK_S_COMPANION])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, glowering_rogon());
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

    let rogon = on_battlefield(&engine, P0, glowering_rogon()).unwrap();
    assert_eq!(counters_on(&engine, rogon, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, rogon), (4, 4));
}
