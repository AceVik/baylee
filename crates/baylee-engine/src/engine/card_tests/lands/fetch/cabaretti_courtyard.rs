//! `cards/lands/fetch/cabaretti_courtyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cabaretti Courtyard prints `When this land enters, sacrifice it. When you do, search your library for a basic Mountain, Forest, or Plains card, put it onto the battlefield tapped, then shuffle and you gain 1 life.`
/// The enter trigger sacrifices the land. The "When you do" is a reflexive triggered ability (CR 603.12), a stack object of its own, which is on the stack
/// before anything is searched. It then fetches a basic Forest onto the battlefield tapped and gains 1 life.
#[test]
fn cabaretti_courtyard_sacrifices_on_etb_to_fetch_basic_land_tapped_and_gain_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[cabaretti_courtyard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let initial_life = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, cabaretti_courtyard());

    // The enter trigger resolves first. What it leaves behind is the
    // reflexive ability, on the stack and from the land, with nothing yet
    // searched.
    pass_until(&mut engine, |e| {
        e.state()
            .object(land)
            .is_some_and(|o| o.zone == Zone::Graveyard)
    });
    let waiting: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.ability)
                .is_some_and(|loc| {
                    loc.index == baylee_core::ids::AbilityRef::SYNTHETIC && loc.source == land
                })
        })
        .collect();
    assert_eq!(
        waiting.len(),
        1,
        "one reflexive ability from the land is on the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        initial_life,
        "nothing has resolved from it yet"
    );

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

    assert!(in_graveyard(&engine, p0, cabaretti_courtyard()).is_some());
    assert_eq!(
        engine.state().object(chosen).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&engine, chosen));
    assert_eq!(engine.state().players[0].life, initial_life + 1);
}
