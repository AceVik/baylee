//! `cards/lands/gates/baldur_s_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Baldur's Gate: `{2}, {T}: Add X mana of any one color, where X is the
/// number of other Gates you control.`
///
/// Two readings meet on one card, and either alone would be wrong:
///
/// - **`Other` is a filter about the card asking**, and `eval::amount` hands
///   `matches` the source object, so the Gate does not count itself. The
///   condition vocabulary refuses `Other` outright for exactly the reason it
///   works here — a condition has no source to be another *than*.
/// - **"any **one** color"** is one pick for the whole amount, which is
///   `mana_choice_dynamic`. `mana_combination` would ask twice and let the
///   two answers differ, which is a different card (the filter cycle above).
#[test]
fn baldurs_gate_counts_the_other_gates_and_asks_once_for_them_all() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(935, forest())
        .battlefield(
            0,
            &[
                baldurs_gate(),
                azorius_guildgate(),
                boros_guildgate(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[azorius_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gate = on_battlefield(&engine, p0, baldurs_gate()).expect("the land is on the table");

    tap_mana_except(&mut engine, p0, gate);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gate,
                ability_index: 1,
            },
        )
        .expect("two Forests pay the {2}");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("one pick for the whole amount: {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "any one *color*, so all five are offered");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("a colour the engine offered");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "and no second question, which is what `any one color` means: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two *other* Gates of mine — the Gate itself does not count, and \
         neither does the opponent's"
    );
}
