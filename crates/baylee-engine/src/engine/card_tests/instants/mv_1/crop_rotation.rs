//! `cards/instants/mv_1/crop_rotation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crop Rotation — {G} instant: "As an additional cost to cast this spell,
/// sacrifice a land. Search your library for a land card, put that card onto
/// the battlefield, then shuffle."
///
/// This played the whole card once before, until the transcoder was caught
/// dropping the additional cost: the played test was green over a one-mana
/// tutor that sacrificed nothing, because its graveyard assertion sat inside
/// `if let Some(paid) = sacrificed`. Here nothing is conditional (#52). The
/// cast must stop at the `CostSacrifice` question, that question must offer
/// the caster's lands and not the Elves beside them, and the land named must
/// be in the graveyard before the spell resolves.
#[test]
fn crop_rotation_sacrifices_a_land_to_put_a_land_from_the_library_onto_the_battlefield() {
    use crate::choice::ChoicePrompt;

    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[crop_rotation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, crop_rotation());
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the additional cost asks which land, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert_eq!((min, max), (1, 1), "\"sacrifice a land\" is one land");
    assert_eq!(
        options.len(),
        2,
        "both Forests and nothing else: {options:?}"
    );
    assert!(!options.contains(&elves), "a creature is not a land");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the land is paid as the spell is cast, before anything resolves"
    );
    assert!(on_stack(&engine, crop_rotation()).is_some());

    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert!(!options.is_empty() && options.iter().all(|o| library.contains(o)));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let found = on_battlefield(&engine, p0, plains()).expect("the found land arrived");
    assert!(
        !is_tapped(&engine, found),
        "\"put that card onto the battlefield\" says nothing of tapped"
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
    assert!(in_graveyard(&engine, p0, crop_rotation()).is_some());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}
