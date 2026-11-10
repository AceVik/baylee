//! `cards/lands/utility/karakas.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Karakas returns a legendary creature — an opponent's, to *their* hand.
///
/// Two halves that a board with one creature on it cannot tell apart: the
/// filter (a nonlegendary creature beside it must not be offered) and the
/// destination (CR 110.5a sends a permanent to its **owner's** hand, which
/// is not the player pressing the button).
#[test]
fn karakas_returns_only_a_legendary_creature_and_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4_701, forest())
        .battlefield(0, &[karakas()])
        .battlefield(1, &[ragavan_nimble_pilferer(), ignoble_hierarch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, karakas()).expect("Karakas is on the table");
    let legend =
        on_battlefield(&engine, p1, ragavan_nimble_pilferer()).expect("their legend is too");
    let plain = on_battlefield(&engine, p1, ignoble_hierarch()).expect("and their nonlegend");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the bounce is offered on an untapped Karakas");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&legend),
        "a legendary creature is what it returns"
    );
    assert!(
        !options.contains(&plain),
        "a nonlegendary creature was offered to Karakas: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .expect("a creature the menu named is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, ragavan_nimble_pilferer()).is_none(),
        "the legend is still on the battlefield"
    );
    assert!(
        in_hand(&engine, p1, ragavan_nimble_pilferer()).is_some(),
        "CR 110.5a: it goes to its owner's hand, not to the hand of whoever \
         pressed the button"
    );
    assert!(
        in_hand(&engine, p0, ragavan_nimble_pilferer()).is_none(),
        "and it is not in mine"
    );
    assert!(is_tapped(&engine, land), "Karakas paid its own {{T}}");
}
