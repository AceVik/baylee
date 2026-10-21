//! Vexing Arcanix — `{4}` Artifact (Ice Age)
//!
//! `{3}`, `{T}`: Target player chooses a card name, then reveals the top card
//! of their library. If that card has the chosen name, the player puts it into
//! their hand. Otherwise, the player puts it into their graveyard and Vexing
//! Arcanix deals 2 damage to them.
//! (`Coverage::Partial`: omits the 2 damage on a wrong guess.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn vexing_arcanix() -> CardIndex {
    card_index("2fd5b7e5-496c-4f6d-85e0-256afb7544d1")
}

#[test]
fn vexing_arcanix_puts_top_card_into_hand_on_matching_name_and_into_graveyard_on_mismatch() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[vexing_arcanix()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
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

    // Cast Vexing Arcanix ({4}).
    let va_hand = in_hand(&engine, p0, vexing_arcanix()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: va_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let va = on_battlefield(&engine, p0, vexing_arcanix()).unwrap();

    // Branch 1: correct guess (Forest, since p1's library is all Forests) -> top card goes to p1's hand.
    let top_match = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: va,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseCardName {
                card: forest(),
                face: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(top_match).unwrap().zone,
        Zone::Hand,
        "matching top card goes to target player's hand"
    );

    // Advance to p0's next main phase so Vexing Arcanix untaps.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });

    // Branch 2: wrong guess (Llanowar Elves against a library of Forests) -> top card goes to p1's graveyard.
    let top_miss = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: va,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseCardName {
                card: llanowar_elves(),
                face: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(top_miss).unwrap().zone,
        Zone::Graveyard,
        "non-matching top card goes to target player's graveyard"
    );
}
