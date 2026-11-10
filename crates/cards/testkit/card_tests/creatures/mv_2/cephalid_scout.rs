//! `cards/creatures/mv_2/cephalid_scout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Cephalid Scout` prints `KeywordSet::FLYING` and an activated ability with cost
/// `{{2}}{{U}}, Sacrifice a land` to draw a card under `Coverage::Implemented`.
/// Activating the ability by sacrificing an Island and paying `{{2}}{{U}}` verifies that
/// `Pending::ChooseCards` with `ChoicePrompt::CostSacrifice` is prompted and a card is drawn.
#[test]
fn cephalid_scout_sacrifices_land_to_draw() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1606, forest())
        .battlefield(
            0,
            &[cephalid_scout(), island(), island(), island(), island()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scout = on_battlefield(&engine, p0, cephalid_scout()).expect("scout is on battlefield");
    assert!(keywords(&engine, scout).contains(KeywordSet::FLYING));

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, cephalid_scout(), 0);

    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostSacrifice,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected land sacrifice prompt for Cephalid Scout");
    };
    assert!(
        !options.is_empty(),
        "controlled lands are available to sacrifice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "a card was drawn into hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "a card was taken from library"
    );
    assert!(
        in_graveyard(&engine, p0, island()).is_some(),
        "sacrificed land is in graveyard"
    );
}
