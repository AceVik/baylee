//! `cards/lands/utility/mirrodin_s_core.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mirrodin's Core, which is the only land in the pool that fills itself.
///
/// Both counter doors on one card and one turn apart, which is why it is
/// worth a test of its own: `{T}: Put a charge counter on this land` is an
/// **effect** and goes through `replacement::put_counters`, while
/// `{T}, Remove a charge counter from this land` is a **cost** and goes
/// through `replacement::remove_counters`. A card that could do only the
/// first would be a land that fills up and never spends.
#[test]
fn mirrodin_s_core_fills_itself_and_then_spends_what_it_put_on() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(544, forest())
        .hand(0, &[mirrodin_s_core()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrodin_s_core());
    assert!(!entered_tapped(&engine, land), "the Core enters untapped");
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "and empty"
    );

    // Ability 1 is not a mana ability, so it uses the stack (CR 605.1).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the Core may charge itself");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, land),
        "it charged itself with its own {{T}}"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 2,
            },
        )
        .expect("the counter it put on itself is the one it spends");
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("any colour is a choice: {:?}", engine.pending())
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("a colour the engine offered");

    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "and it is empty again"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
    );
}
