//! `cards/lands/cascading_cataracts.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cascading Cataracts: `{5}, {T}: Add five mana in any combination of
/// colors.`
///
/// The fixed-number form of the same sentence, and the one that says the
/// number is not the counter-X machinery wearing a hat: nothing announces
/// anything here, the cost has no counter in it at all, and five picks still
/// come out — `Amount::Fixed(5)` with `combination: true`.
///
/// Five colours offered rather than two, which is the other half of the
/// reading: `ALL_MANA_COLORS` against the `and/or` pair above.
#[test]
fn cascading_cataracts_asks_five_times_and_offers_every_colour() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(913, forest())
        .battlefield(
            0,
            &[
                cascading_cataracts(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land =
        on_battlefield(&engine, p0, cascading_cataracts()).expect("the land is on the table");

    tap_mana_except(&mut engine, p0, land);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("five Forests pay the {5}");
    assert!(
        matches!(engine.pending(), Pending::ChooseColor { .. }),
        "no counter is announced here, so the colour is the first question: \
         {:?}",
        engine.pending()
    );

    for (i, color) in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ]
    .into_iter()
    .enumerate()
    {
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("pick {i} of five: {:?}", engine.pending())
        };
        assert_eq!(
            options.len(),
            5,
            "pick {i}: `any combination of colors` is all five, every time"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(color))
            .expect("a colour the engine offered");
    }

    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "five picks and no sixth: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert_eq!(
            pool.available(color),
            1,
            "five mana, one of each, which is what five separate picks buys"
        );
    }
}
