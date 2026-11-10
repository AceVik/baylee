//! `cards/lands/utility/command_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Command Bridge: "sacrifice it unless you **tap** an untapped permanent
/// you control."
///
/// The same "unless" as the Karoo cycle with the other asking part, and the
/// row that says the two families are one rule: the price is a `TapOther`
/// rather than a `ReturnToHand`, and nothing else about the sentence
/// changes. What it also pins is CR 118.3 on the menu — a permanent that is
/// already tapped is not an answer, which is what the card's own word
/// "untapped" says and what `cost_wizard::options` supplies.
#[test]
fn a_land_that_costs_a_tap_keeps_itself_when_something_untapped_is_named() {
    let p0 = PlayerId::new(0);
    let bridge = card_index("87c8e1ed-258a-4a89-bcc6-211405e49692");
    let mut engine = Duel::new(930, forest())
        .battlefield(0, &[forest(), island()])
        .hand(0, &[bridge])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // One of the two is spent, so the menu has to be shorter than the board.
    let island_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == island())
        })
        .expect("the Island is on the battlefield");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: island_id })
        .unwrap();

    let land = play_land(&mut engine, p0, bridge);
    let (options, prompt) = reach_the_unless_question(&mut engine, land).expect("it asks");
    assert_eq!(prompt, ChoicePrompt::CostTap);
    assert!(
        !options.contains(&island_id),
        "a tapped permanent cannot be tapped to pay (CR 118.3)"
    );
    let paid = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .unwrap();

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "the land was paid for and stays"
    );
    assert!(
        engine
            .state()
            .object(paid)
            .is_some_and(|o| o.status.contains(Status::TAPPED)),
        "and what paid is tapped"
    );
}
