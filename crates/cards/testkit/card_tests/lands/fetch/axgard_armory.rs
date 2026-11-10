//! `cards/lands/fetch/axgard_armory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Axgard Armory prints `This land enters tapped`, `{{T}}: Add {{W}}`, and
/// `{{1}}{{R}}{{R}}{{W}}, {{T}}, Sacrifice this land: Search your library for an Aura card and/or an Equipment card, reveal them, put them into your hand, then shuffle.`
/// The card is marked `Coverage::Implemented`.
/// When played, it enters tapped, untaps on the subsequent turn, and sacrifices with floating mana to search the library for an Equipment card into hand.
#[test]
fn axgard_armory_enters_tapped_and_sacrifices_to_search_equipment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, lightning_greaves())
        .battlefield(0, &[mountain(), mountain(), plains(), forest()])
        .hand(0, &[axgard_armory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let armory = play_land(&mut engine, p0, axgard_armory());
    assert!(entered_tapped(&engine, armory));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, armory));

    tap_mana_except(&mut engine, p0, armory);
    activate(&mut engine, p0, axgard_armory(), 1);
    assert!(in_graveyard(&engine, p0, axgard_armory()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search prompt");
    };
    assert!(!options.is_empty());
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(chosen).unwrap().zone, Zone::Hand);
}
