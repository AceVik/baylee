//! `cards/sorceries/mv_4/skyshroud_claim.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn skyshroud_claim_puts_up_to_two_forests_onto_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("376c9d3f-21d3-4251-bb6a-026fa9e1b0e1")])
        .battlefield(0, &[forest(); 4])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("376c9d3f-21d3-4251-bb6a-026fa9e1b0e1"),
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards for Skyshroud Claim");
    };
    assert_eq!(min, 0);
    assert_eq!(max, 2);
    let picked = vec![options[0], options[1]];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: picked.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    for f in picked {
        assert_eq!(engine.state().object(f).unwrap().zone, Zone::Battlefield);
        assert!(
            !is_tapped(&engine, f),
            "Skyshroud Claim puts Forests onto the battlefield untapped"
        );
    }
}
