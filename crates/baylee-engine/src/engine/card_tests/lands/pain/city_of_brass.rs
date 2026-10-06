//! `cards/lands/pain/city_of_brass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// City of Brass charges a life for *becoming tapped*, and does it on the
/// stack.
///
/// That is the difference worth a test rather than the damage itself. The
/// mana ability uses no stack (CR 605.3b), so the mana is in the pool the
/// instant it is pressed; the damage is an ordinary triggered ability that
/// goes on the stack and resolves after. A card that wrote the damage into
/// the ability's cost instead would pass every assertion about life and be
/// wrong about when — an opponent may respond to this trigger, and may not
/// respond to a cost.
#[test]
fn city_of_brass_deals_its_damage_as_a_trigger_and_not_as_a_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4_810, forest())
        .battlefield(0, &[city_of_brass()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, city_of_brass()).expect("the City is on the table");
    let before = engine.state().players[0].life;

    // Index 1 and not 0: the card prints the trigger first, so the mana
    // ability is the second entry and an index read off the reading order
    // rather than off the card would have asked to activate a trigger.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the mana ability is offered on an untapped City");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(
        options.len(),
        5,
        "five colours and colorless is none of them: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was on the menu");

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the mana ability used no stack, so the mana is already there"
    );
    assert!(
        is_tapped(&engine, land),
        "and the {{T}} is what paid for it"
    );
    assert_eq!(
        engine.state().players[0].life,
        before,
        "the damage has not happened yet: it is a trigger waiting to resolve, \
         which is the window an opponent may act in"
    );
    assert!(
        !stack_is_empty(&engine),
        "\"whenever this land becomes tapped\" is a triggered ability and \
         triggered abilities use the stack (CR 603.3)"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before - 1,
        "one damage to the land's controller, once the trigger resolves"
    );
}
