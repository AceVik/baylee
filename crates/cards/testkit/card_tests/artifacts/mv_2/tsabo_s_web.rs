//! `cards/artifacts/mv_2/tsabo_s_web.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tsabo_s_web() -> CardIndex {
    card_index("96a4e515-2b4e-4c2d-aa1a-963597484723")
}

fn karakas() -> CardIndex {
    card_index("59119143-c0fa-49dd-adf0-e2fd3029c48b")
}

fn ragavan_nimble_pilferer() -> CardIndex {
    card_index("37108cd4-bbab-4ce3-9ed6-f60e8422e703")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Tsabo's Web draws exactly the top card when its enters trigger resolves.
#[test]
fn tsabo_s_web_draws_a_card_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[tsabo_s_web()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let expected = *library.last().expect("the deck has a top card");
    cast_from_hand(&mut engine, p0, tsabo_s_web());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, tsabo_s_web()).is_some());
    assert_eq!(library_size(&engine, p0), library.len() - 1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&expected),
        "the card that was on top is now in hand"
    );
}

/// A land with a non-mana activated ability stays tapped; a pure mana land and
/// a nonland permanent untap normally in the same untap step.
#[test]
fn tsabo_s_web_holds_only_lands_with_non_mana_activated_abilities_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tsabo_s_web(), karakas(), forest(), grizzly_bears()])
        .battlefield(1, &[ragavan_nimble_pilferer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let web = on_battlefield(&engine, p0, tsabo_s_web()).expect("the Web is seated");
    let utility = on_battlefield(&engine, p0, karakas()).expect("Karakas is seated");
    let mana_only = on_battlefield(&engine, p0, forest()).expect("the Forest is seated");
    let creature = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are seated");
    let legend = on_battlefield(&engine, p1, ragavan_nimble_pilferer())
        .expect("Ragavan is a legal Karakas target");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: utility,
                ability_index: 1,
            },
        )
        .expect("Karakas's non-mana ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .expect("the legendary creature is a legal target");
    pass_until(&mut engine, stack_is_empty);
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: mana_only })
        .expect("the Forest taps only for mana");
    assert!(is_tapped(&engine, utility));
    assert!(is_tapped(&engine, mana_only));

    // Set the nonland fixture after the opponent's untap step, then cross back
    // to the Web controller's next main phase.
    reach_their_main_phase(&mut engine, p1);
    engine
        .dev_state_mut(p0)
        .expect("the harness may set the tapped fixture")
        .object_mut(creature)
        .expect("the Bears are still seated")
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();
    assert!(walk_to_own_main(&mut engine, p0));

    assert!(
        is_tapped(&engine, utility),
        "Karakas has a non-mana activated ability beside its mana ability"
    );
    assert!(
        !is_tapped(&engine, mana_only),
        "a Forest's intrinsic mana ability is not enough"
    );
    assert!(
        !is_tapped(&engine, creature),
        "the Web's filter only names lands"
    );
    assert!(
        !is_tapped(&engine, web),
        "the artifact itself is not a land"
    );
}
