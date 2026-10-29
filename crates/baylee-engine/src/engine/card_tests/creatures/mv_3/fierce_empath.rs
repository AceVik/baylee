//! `cards/creatures/mv_3/fierce_empath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn fierce_empath_searches_library_for_creature_with_mana_value_six_or_greater() {
    let empath = card_index("5104053a-d394-4b00-82a4-60fd9f051a6e");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, index::CRAW_WURM)
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[empath])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, empath);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for SearchLibrary");
    };
    let wurm_id = *options
        .iter()
        .find(|&&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == index::CRAW_WURM)
        })
        .expect("Craw Wurm (MV 6) should be offered");
    assert!(
        !options.iter().any(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == index::GRIZZLY_BEARS)
        }),
        "Grizzly Bears (MV 2) must not be offered"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, index::CRAW_WURM).is_some());
}
