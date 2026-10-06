//! `cards/lands/restricted/bleachbone_verge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fellwar Stone reads what an opponent's land "could produce" (CR 106.7),
/// and a restriction on activating is not part of that answer.
///
/// Bleachbone Verge adds {B} outright and {W} only while its controller has
/// a Plains or a Swamp. CR 106.7 asks what an ability would produce *if it
/// were to resolve*, says to ignore whether its costs could be paid, and
/// says nothing about activation restrictions — which CR 602.5 puts on
/// beginning the activation, not on the resolution. So the Verge could
/// produce {W} on a board with neither, and this test sets up exactly that
/// board: the opponent's only land is the Verge, and it is the only
/// permanent that could name a colour.
///
/// `Characteristics::from_face` collected the colours from
/// `AbilityDef::Activated { mana_ability: true }` alone, so the Verge
/// offered {B} and nothing else.
#[test]
fn a_conditional_mana_ability_still_says_what_its_land_could_produce() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(733, forest())
        .battlefield(0, &[fellwar_stone()])
        .battlefield(1, &[bleachbone_verge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stone = on_battlefield(&engine, p0, fellwar_stone()).expect("the Stone is on the table");
    // Asked of the offer rather than assumed: `legal.mana_abilities` is the
    // intrinsic path (a basic land's own tap), and a mana ability that runs
    // an effect is offered in `legal.abilities` like any other activation —
    // it skips the stack later, in `start_activation` (CR 605.1).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (_, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == stone)
        .expect("the Stone's mana ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: stone,
                ability_index,
            },
        )
        .expect("a mana ability with no cost but the tap");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ManaColor::White),
        "the Verge could produce {{W}} however the condition stands: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black),
        "and {{B}} outright: {options:?}"
    );
}
