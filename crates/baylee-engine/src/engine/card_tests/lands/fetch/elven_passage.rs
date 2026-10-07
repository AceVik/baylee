//! `cards/lands/fetch/elven_passage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elven Passage prints `{{T}}, Pay 1 life, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. You may behold an Elf. If you do, untap that land.`
/// The card is marked `Coverage::Partial` because the behold mechanic and conditional untapping are unsupported and omitted.
/// Activating its ability pays 1 life, sacrifices the land, and fetches a basic forest onto the battlefield tapped.
#[test]
fn elven_passage_sacrifices_and_pays_life_to_fetch_basic_land_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[elven_passage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let initial_life = engine.state().players[0].life;
    let passage = play_land(&mut engine, p0, elven_passage());
    assert!(!is_tapped(&engine, passage));

    activate(&mut engine, p0, elven_passage(), 0);
    assert!(in_graveyard(&engine, p0, elven_passage()).is_some());
    assert_eq!(engine.state().players[0].life, initial_life - 1);

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

    assert_eq!(
        engine.state().object(chosen).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&engine, chosen));
}
